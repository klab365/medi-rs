use medi_rs::error::Error;
use medi_rs::traits::IntoReq;
use medi_rs::{bus::BusBuilder, error::Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[tokio::test]
async fn send_should_return_correct_pong() {
    let bus = BusBuilder::default().add_req_handler(print_ping).build();

    let pong = bus.send(Ping("Ping".to_string())).await.unwrap();

    assert_eq!(pong.0, "Pong: Ping");
}

#[tokio::test]
async fn send_should_return_correct_multiple_pong_without_multithreading() {
    let bus = BusBuilder::default().add_req_handler(print_ping).build();

    let pong = bus.send(Ping("Ping".to_string())).await.unwrap();
    assert_eq!(pong.0, "Pong: Ping");

    let pong = bus.send(Ping("Ping2".to_string())).await.unwrap();
    assert_eq!(pong.0, "Pong: Ping2");
}

#[tokio::test]
async fn send_should_return_correct_return_values_when_multithreading() {
    let bus = BusBuilder::default().add_req_handler(print_ping).build();

    let mut handlers = vec![];
    let bus = Arc::new(bus);
    for i in 0..100 {
        let bus = bus.clone();
        let handler = tokio::spawn(async move {
            let pong = bus.send(Ping(format!("Ping{}", i))).await.unwrap();
            assert_eq!(pong.0, format!("Pong: Ping{}", i));
        });

        handlers.push(handler);
    }

    for handler in handlers {
        handler.await.unwrap();
    }
}

#[tokio::test]
async fn send_should_return_error_when_handler_returns_error() {
    let bus = BusBuilder::default().add_req_handler(print_ping_with_error).build();

    let result = bus.send(Ping("Ping".to_string())).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    match &err {
        Error::Other(err) => {
            let err = err.downcast_ref::<PingPongError>().unwrap();
            assert_eq!(err, &PingPongError::Error1);
        }
        _ => panic!("Error type is not correct"),
    }
    assert_eq!(err.to_string(), "Error");
}

fn print_ping(id: Ping) -> Result<Pong> {
    Ok(Pong(format!("Pong: {}", id.0)))
}

fn print_ping_with_error(_id: Ping) -> Result<Pong> {
    Err(Error::Other(Box::new(PingPongError::Error1)))
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum PingPongError {
    #[error("Error")]
    Error1,
}

#[derive(Serialize, Deserialize)]
struct Ping(String);
impl IntoReq<Pong> for Ping {}

#[derive(Debug, Serialize, Deserialize)]
struct Pong(String);
