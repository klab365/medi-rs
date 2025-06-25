use std::sync::Arc;

use medi_rs::{Bus, FromResources, IntoCommand};
use medi_rs_macros::{MediCommand, MediRessource};
use tokio::sync::Mutex;

#[tokio::test]
async fn send_should_return_error() {
    let bus = Bus::builder().add_req_handler(error_handler).build().unwrap();

    let res = bus.send(BasicRequest).await;

    // assert
    match res {
        Ok(_) => panic!("Expected error, got {:?}", res),
        Err(err) => {
            let my_error = err.get_handler_error::<CustomError>().unwrap();

            match my_error {
                CustomError::Basic(content) => assert_eq!(content, "Error1"),
                _ => panic!("Expected CustomError::Basic, got {:?}", my_error),
            }
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
    Err(CustomError::Basic("Error1".to_string()))
}

async fn error_handler1(state: AppState, _req: BasicRequest) -> Result<(), CustomError> {
    state.list.lock().await.push("test".to_string());
    Err(CustomError::Basic("Error2".to_string()))
}

#[derive(MediCommand)]
struct BasicRequest;

#[derive(Debug, Clone, MediRessource)]
pub struct AppState {
    pub list: Arc<Mutex<Vec<String>>>,
}

#[derive(thiserror::Error, Debug)]
enum CustomError {
    #[error("Error")]
    Basic(String),

    #[error("Bus error")]
    BusError(#[from] medi_rs::Error),
}
