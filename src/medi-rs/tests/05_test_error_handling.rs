#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
#[medi_command(error_type = CustomError)]
struct BasicRequest;

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
enum CustomError {
    #[error("basic error: {0}")]
    Basic(&'static str),
}

#[medi_handler]
async fn error_handler(_: BasicRequest) -> Result<(), CustomError> {
    Err(CustomError::Basic("Error1"))
}

medi_module! { manifest error_manifest; commands { BasicRequest => error_handler; } }
mediator! {
    pub struct ErrorMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [error_manifest];
    }
}

#[tokio::test]
async fn send_should_return_the_typed_handler_error() {
    let error = ErrorMediator::new().send(BasicRequest).await.unwrap_err();
    assert_eq!(error, CustomError::Basic("Error1"));
}
