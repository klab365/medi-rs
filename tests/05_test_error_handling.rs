use medi_rs::{
    Bus, FromResources, {IntoHandlerError, IntoReq},
};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn send_should_return_error() {
    let bus = Bus::builder().add_req_handler(error_handler).build();

    let res = bus.send(BasicRequest).await;

    // assert
    match res {
        Ok(_) => panic!("Expected error, got {:?}", res),
        Err(err) => match err {
            medi_rs::Error::Handler(handler_error) => {
                let my_error = handler_error.get::<Error>().unwrap();
                assert!(matches!(my_error, Error::Error(_)));
            }
            _ => panic!("Expected HandlerError, got {:?}", err),
        },
    }
}

#[tokio::test]
async fn send_should_return_error_when_handler_not_found() {
    let bus = Bus::builder().build();

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
    let bus = Bus::builder().add_req_handler(error_handler1).build();

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

async fn error_handler(_req: BasicRequest) -> Result<(), Error> {
    Err(Error::Error("Error".to_string()))
}

async fn error_handler1(_state: AppState, _req: BasicRequest) -> Result<(), Error> {
    Err(Error::Error("Error".to_string()))
}

#[derive(Debug, Clone)]
struct AppState {
    pub _list: Arc<Mutex<Vec<String>>>,
}
impl FromResources for AppState {}

struct BasicRequest;
impl IntoReq<()> for BasicRequest {}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Error")]
    Error(String),

    #[error("Bus error")]
    BusError(#[from] medi_rs::Error),
}

impl IntoHandlerError for Error {}
