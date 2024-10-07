use std::sync::Arc;

use medi_rs::traits::Result;
use medi_rs::{bus::Bus, traits::IntoReq};
use serde::{Deserialize, Serialize};

#[tokio::test]
async fn send_should_return_correct_pong() {
    let mut bus = Bus::new();
    bus.add_req_handler(print_ping);

    let pong = bus.send(Ping("Ping".to_string())).await.unwrap();

    assert_eq!(pong.0, "Pong: Ping");
}

#[tokio::test]
async fn send_should_return_correct_multiple_pong_without_multithreading() {
    let mut bus = Bus::new();
    bus.add_req_handler(print_ping);

    let pong = bus.send(Ping("Ping".to_string())).await.unwrap();
    assert_eq!(pong.0, "Pong: Ping");

    let pong = bus.send(Ping("Ping2".to_string())).await.unwrap();
    assert_eq!(pong.0, "Pong: Ping2");
}

#[tokio::test]
async fn send_should_return_correct_return_values_when_multithreading() {
    let mut bus = Bus::new();
    bus.add_req_handler(print_ping);

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

fn print_ping(id: Ping) -> Result<Pong> {
    Ok(Pong(format!("Pong: {}", id.0)))
}

#[derive(Serialize, Deserialize)]
struct Ping(String);
impl IntoReq<Pong> for Ping {}

#[derive(Serialize, Deserialize)]
struct Pong(String);
