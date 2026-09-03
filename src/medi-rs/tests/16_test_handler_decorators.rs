#![cfg(feature = "tokio")]

use core::convert::Infallible;
use std::sync::Mutex;

use medi_rs::{DecoratorNext, MediCommand, medi_handler, medi_module, mediator};

static CALLS: Mutex<Vec<&str>> = Mutex::new(Vec::new());

async fn global_logging<Command, Next>(command: Command, next: Next) -> Result<Next::Response, Next::Error>
where
    Command: Send,
    Next: DecoratorNext<Command>,
{
    CALLS.lock().unwrap().push("global:before");
    let result = next.call(command).await;
    CALLS.lock().unwrap().push("global:after");
    result
}

async fn logging(
    command: DecoratedCommand,
    next: impl DecoratorNext<DecoratedCommand, Response = &'static str, Error = Infallible>,
) -> Result<&'static str, Infallible> {
    CALLS.lock().unwrap().push("logging:before");
    let result = next.call(command).await;
    CALLS.lock().unwrap().push("logging:after");
    result
}

async fn validation(
    command: DecoratedCommand,
    next: impl DecoratorNext<DecoratedCommand, Response = &'static str, Error = Infallible>,
) -> Result<&'static str, Infallible> {
    CALLS.lock().unwrap().push("validation:before");
    let result = next.call(command).await;
    CALLS.lock().unwrap().push("validation:after");
    result
}

#[derive(MediCommand)]
#[medi_command(return_type = &'static str)]
struct DecoratedCommand;

#[medi_handler(decorators = [logging, validation])]
async fn handle_decorated_command(_: DecoratedCommand) -> Result<&'static str, Infallible> {
    CALLS.lock().unwrap().push("handler");
    Ok("handled")
}

medi_module! {
    manifest decorated_manifest;
    commands { DecoratedCommand => handle_decorated_command; }
}

mediator! {
    struct DecoratedMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [decorated_manifest];
        decorators: [global_logging];
    }
}

#[tokio::test]
async fn function_decorators_receive_and_forward_the_command() {
    CALLS.lock().unwrap().clear();

    assert_eq!(
        DecoratedMediator::new().send(DecoratedCommand).await.unwrap(),
        "handled"
    );
    assert_eq!(
        *CALLS.lock().unwrap(),
        [
            "global:before",
            "logging:before",
            "validation:before",
            "handler",
            "validation:after",
            "logging:after",
            "global:after"
        ]
    );
}
