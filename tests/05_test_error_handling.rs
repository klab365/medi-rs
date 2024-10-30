use medi_rs::{
    Bus, FromResources, {IntoCommand, IntoHandlerError},
};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn send_should_return_error() {
    let bus = Bus::builder().add_req_handler(error_handler).build().unwrap();

    let res = bus.send(BasicRequest).await;

    // assert
    match res {
        Ok(_) => panic!("Expected error, got {:?}", res),
        Err(err) => {
            let my_error = err.get_handler_error::<CustomError>().unwrap();
            assert!(matches!(my_error, CustomError::Basic(_)));
        }
    }
}

#[tokio::test]
async fn send_should_return_error_when_handler_not_found() {
    let bus = Bus::builder().build().unwrap();

    let res = bus.send(BasicRequest).await;

    // assert
    match res {
        Ok(_) => panic!("Expected error, got {:?}", res),
        Err(err) => match err {
            medi_rs::Error::HandlerNotFound => (),
            _ => panic!("Expected HandlerNotFound, got {:?}", err),
        },
    }
}

#[tokio::test]
async fn send_should_return_error_when_no_resource_found() {
    let bus = Bus::builder().add_req_handler(error_handler1).build().unwrap();

    let res = bus.send(BasicRequest).await;

    // assert
    match res {
        Ok(_) => panic!("Expected error, got {:?}", res),
        Err(err) => match err {
            medi_rs::Error::ResourceNotFound => (),
            _ => panic!("Expected ResourceNotFound, got {:?}", err),
        },
    }
}

async fn error_handler(_req: BasicRequest) -> Result<(), CustomError> {
    Err(CustomError::Basic("Error".to_string()))
}

async fn error_handler1(_state: AppState, _req: BasicRequest) -> Result<(), CustomError> {
    Err(CustomError::Basic("Error".to_string()))
}

#[derive(Debug, Clone)]
struct AppState {
    pub _list: Arc<Mutex<Vec<String>>>,
}
impl FromResources for AppState {}

struct BasicRequest;
impl IntoCommand<()> for BasicRequest {}

#[derive(thiserror::Error, Debug)]
enum CustomError {
    #[error("Error")]
    Basic(String),

    #[error("Bus error")]
    BusError(#[from] medi_rs::Error),
}

impl IntoHandlerError for CustomError {}
