#![cfg(feature = "tokio")]

use std::sync::{Arc, Mutex};

use medi_rs::{DecoratorNext, MediCommand, Result, medi_handler, medi_module, mediator};

mod event_routing {
    use super::*;
    use tokio::sync::Notify;

    static CALLS: Mutex<Vec<&str>> = Mutex::new(Vec::new());
    static COMPLETION: Notify = Notify::const_new();

    #[derive(Clone)]
    struct First;
    #[derive(Clone)]
    struct Second;

    #[medi_handler]
    async fn first(mediator: &Mediator, _: First) -> Result<()> {
        CALLS.lock().unwrap().push("first");
        mediator.publish(Second).await
    }

    #[medi_handler]
    async fn second(_: Second) -> Result<()> {
        CALLS.lock().unwrap().push("second");
        COMPLETION.notify_one();
        Ok(())
    }

    medi_module! {
        manifest manifest;
        events { First => [first]; Second => [second]; }
    }
    mediator! {
        struct Mediator {
            event_queue_capacity: 2;
            event_workers: 1;
            modules: [manifest];
        }
    }

    #[tokio::test]
    async fn queued_events_wait_for_start_and_route_recursive_events() {
        CALLS.lock().unwrap().clear();
        let mediator = Box::leak(Box::new(Mediator::new()));

        mediator.publish(First).await.unwrap();
        assert!(CALLS.lock().unwrap().is_empty());

        let completed = COMPLETION.notified();
        mediator.start();
        tokio::time::timeout(std::time::Duration::from_secs(1), completed)
            .await
            .expect("recursive event should be dispatched");
        assert_eq!(*CALLS.lock().unwrap(), ["first", "second"]);
    }
}

mod command_decorators {
    use super::*;

    #[derive(Clone)]
    struct Observed(Arc<Mutex<Vec<String>>>);

    #[derive(MediCommand)]
    #[medi_command(return_type = String, error_type = medi_rs::Error)]
    struct Command(String);

    async fn prefix(
        mut command: Command,
        next: impl DecoratorNext<Command, Response = String, Error = medi_rs::Error>,
    ) -> Result<String> {
        command.0 = format!("prefix:{}", command.0);
        next.call(command).await
    }

    async fn reject_empty(
        command: Command,
        next: impl DecoratorNext<Command, Response = String, Error = medi_rs::Error>,
    ) -> Result<String> {
        if command.0 == "prefix:" {
            return Err(medi_rs::Error::EventPublishingError);
        }
        next.call(command).await
    }

    #[medi_handler(decorators = [prefix, reject_empty])]
    async fn handle(observed: Observed, command: Command) -> Result<String> {
        observed.0.lock().unwrap().push(command.0.clone());
        Ok(command.0)
    }

    medi_module! {
        manifest manifest;
        resources { Observed; }
        commands { Command => handle; }
    }
    mediator! {
        struct Mediator {
            event_queue_capacity: 1;
            event_workers: 1;
            modules: [manifest];
        }
    }

    #[tokio::test]
    async fn decorators_transform_commands_and_can_skip_resource_handlers() {
        let observed = Observed(Arc::new(Mutex::new(Vec::new())));
        let mediator = Mediator::new(observed.clone());

        assert_eq!(mediator.send(Command("value".into())).await.unwrap(), "prefix:value");
        assert_eq!(*observed.0.lock().unwrap(), ["prefix:value"]);

        assert_eq!(
            mediator.send(Command(String::new())).await.unwrap_err(),
            medi_rs::Error::EventPublishingError
        );
        assert_eq!(*observed.0.lock().unwrap(), ["prefix:value"]);
    }
}
