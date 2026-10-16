#![cfg(any(feature = "tokio", feature = "embassy"))]

use core::convert::Infallible;
use medi_rs::{DispatchError, MediCommand, MediContext, SendError, medi_module, mediator};

#[derive(Debug, MediCommand)]
#[medi_command(return_type = u32, error_type = Infallible)]
struct Increment;

#[derive(Debug, MediCommand)]
#[medi_command(return_type = bool, error_type = SendError<Infallible>)]
struct Reenter;

#[derive(Clone)]
struct AddValue(u32);

struct CounterModule {
    value: u32,
}

impl CounterModule {
    async fn increment(
        &mut self,
        _context: &MediContext<'_, CounterMediator>,
        _command: Increment,
    ) -> core::result::Result<u32, Infallible> {
        self.value += 1;
        Ok(self.value)
    }

    async fn reenter(
        &mut self,
        context: &MediContext<'_, CounterMediator>,
        _command: Reenter,
    ) -> core::result::Result<bool, SendError<Infallible>> {
        Ok(matches!(
            context.send(Increment).await,
            Err(SendError::Dispatch(DispatchError::SelfDispatch))
        ))
    }

    async fn add_event(
        &mut self,
        _context: &MediContext<'_, CounterMediator>,
        event: AddValue,
    ) -> core::result::Result<(), Infallible> {
        self.value += event.0;
        Ok(())
    }
}

medi_module! {
    manifest counter_manifest;
    module CounterModule;
    mailbox_capacity: 4;
    commands { Increment => CounterModule::increment; Reenter => CounterModule::reenter; }
    events { AddValue => [CounterModule::add_event]; }
}

mediator! {
    struct CounterMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [counter_manifest];
    }
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn actor_send_and_post_are_routed_to_the_owned_module() {
    let mediator = Box::leak(Box::new(
        CounterMediator::builder()
            .counter_module(CounterModule { value: 0 })
            .build(),
    ));
    mediator.start().expect("actor worker starts");

    mediator.post(Increment).expect("post fits mailbox");
    assert_eq!(mediator.send(Increment).await.unwrap(), 2);
    assert!(mediator.send(Reenter).await.unwrap());
    mediator.publish(AddValue(10)).await.unwrap();
    assert_eq!(mediator.send(Increment).await.unwrap(), 13);

    mediator.shutdown().await.expect("actor worker shuts down");
    assert!(matches!(
        mediator.post(Increment),
        Err(medi_rs::PostError::ModuleUnavailable(_))
    ));
}

#[cfg(feature = "embassy")]
#[test]
fn embassy_actor_uses_fixed_reply_slots() {
    use std::{sync::mpsc, thread, time::Duration};

    let (mediator_tx, mediator_rx) = mpsc::channel();
    thread::spawn(move || {
        let executor = Box::leak(Box::new(embassy_executor::Executor::new()));
        executor.run(|spawner| {
            let mediator: &'static CounterMediator = Box::leak(Box::new(
                CounterMediator::builder()
                    .counter_module(CounterModule { value: 0 })
                    .build(),
            ));
            mediator.start(spawner).expect("actor worker starts");
            mediator_tx.send(mediator).expect("test receives mediator");
        });
    });
    let mediator = mediator_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Embassy executor starts actor worker");
    assert_eq!(futures::executor::block_on(mediator.send(Increment)).unwrap(), 1);
}
