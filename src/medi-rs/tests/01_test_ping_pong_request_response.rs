#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
use std::sync::Arc;

#[derive(MediCommand)]
#[medi_command(return_type = Pong, error_type = medi_rs::Error)]
struct Ping(String);

#[derive(Debug)]
struct Pong(String);

#[medi_handler]
async fn print_ping(id: Ping) -> Result<Pong> {
    Ok(Pong(format!("Pong: {}", id.0)))
}

medi_module! {
    manifest ping_manifest;
    commands { Ping => print_ping; }
}

mediator! {
    pub struct PingMediator {
        event_queue_capacity: 8;
        event_workers: 1;
        modules: [ping_manifest];
    }
}

#[tokio::test]
async fn send_should_return_correct_pong() {
    let mediator = PingMediator::new();
    let pong = mediator.send(Ping("Ping".into())).await.unwrap();
    assert_eq!(pong.0, "Pong: Ping");
}

#[tokio::test]
async fn send_should_return_correct_multiple_pong_without_multithreading() {
    let mediator = PingMediator::new();
    assert_eq!(mediator.send(Ping("Ping".into())).await.unwrap().0, "Pong: Ping");
    assert_eq!(mediator.send(Ping("Ping2".into())).await.unwrap().0, "Pong: Ping2");
}

#[tokio::test]
async fn send_should_return_correct_return_values_when_multithreading() {
    let mediator = Arc::new(PingMediator::new());
    let mut handlers = vec![];
    for i in 0..100 {
        let mediator = Arc::clone(&mediator);
        handlers.push(tokio::spawn(async move {
            let pong = mediator.send(Ping(format!("Ping{i}"))).await.unwrap();
            assert_eq!(pong.0, format!("Pong: Ping{i}"));
        }));
    }
    for handler in handlers {
        handler.await.unwrap();
    }
}
