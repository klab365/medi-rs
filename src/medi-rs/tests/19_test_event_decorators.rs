#![cfg(feature = "tokio")]

use std::sync::Mutex;

use medi_rs::{DecoratorNext, Result, medi_handler, medi_module, mediator};
use tokio::sync::Notify;

static CALLS: Mutex<Vec<&str>> = Mutex::new(Vec::new());
static COMPLETION: Notify = Notify::const_new();

async fn global_logging<Command, Next>(event: Command, next: Next) -> core::result::Result<Next::Response, Next::Error>
where
    Command: Send,
    Next: DecoratorNext<Command>,
{
    CALLS.lock().unwrap().push("global:before");
    let result = next.call(event).await;
    CALLS.lock().unwrap().push("global:after");
    result
}

#[derive(Clone)]
struct Event;

async fn logging(event: Event, next: impl DecoratorNext<Event, Response = (), Error = medi_rs::Error>) -> Result<()> {
    CALLS.lock().unwrap().push("before");
    let result = next.call(event).await;
    CALLS.lock().unwrap().push("after");
    result
}

#[medi_handler(decorators = [logging])]
async fn handle_event(_: Event) -> Result<()> {
    CALLS.lock().unwrap().push("handler");
    COMPLETION.notify_one();
    Ok(())
}

medi_module! {
    manifest event_manifest;
    events { Event => [handle_event]; }
}

mediator! {
    struct EventDecoratorMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [event_manifest];
        decorators: [global_logging];
    }
}

#[tokio::test]
async fn decorators_wrap_event_handlers() {
    CALLS.lock().unwrap().clear();
    let mediator = Box::leak(Box::new(EventDecoratorMediator::new()));
    mediator.start().expect("mediator must start");

    let completed = COMPLETION.notified();
    mediator.publish(Event).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), completed)
        .await
        .expect("decorated event handler should run");

    assert_eq!(
        *CALLS.lock().unwrap(),
        ["global:before", "before", "handler", "after", "global:after"]
    );
}
