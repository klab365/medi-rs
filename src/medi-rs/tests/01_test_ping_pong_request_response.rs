#![cfg(feature = "tokio")]

use core::convert::Infallible;
use medi_rs::{MediCommand, MediContext, medi_module, mediator};
use std::sync::Arc;

#[derive(Debug, MediCommand)]
#[medi_command(return_type = Pong, error_type = Infallible)]
struct Ping(String);

#[derive(Debug)]
struct Pong(String);

struct PingModule;

impl PingModule {
    async fn print_ping(&mut self, _context: &MediContext<'_, PingMediator>, ping: Ping) -> Result<Pong, Infallible> {
        Ok(Pong(format!("Pong: {}", ping.0)))
    }
}

medi_module! {
    manifest ping_manifest;
    module PingModule;
    mailbox_capacity: 128;
    commands { Ping => PingModule::print_ping; }
}

mediator! {
    pub struct PingMediator {
        event_queue_capacity: 8;
        event_workers: 1;
        modules: [ping_manifest];
    }
}

fn new_mediator() -> &'static PingMediator {
    let mediator = Box::leak(Box::new(PingMediator::builder().ping_module(PingModule).build()));
    mediator.start().expect("actor worker starts");
    mediator
}

#[tokio::test]
async fn send_should_return_correct_pong() {
    let mediator = new_mediator();
    let pong = mediator.send(Ping("Ping".into())).await.unwrap();
    assert_eq!(pong.0, "Pong: Ping");
    mediator.shutdown().await.expect("actor worker shuts down");
}

#[tokio::test]
async fn send_should_return_correct_multiple_pong_without_multithreading() {
    let mediator = new_mediator();
    assert_eq!(mediator.send(Ping("Ping".into())).await.unwrap().0, "Pong: Ping");
    assert_eq!(mediator.send(Ping("Ping2".into())).await.unwrap().0, "Pong: Ping2");
    mediator.shutdown().await.expect("actor worker shuts down");
}

#[tokio::test]
async fn send_should_return_correct_return_values_when_multithreading() {
    let mediator = new_mediator();
    let mediator = Arc::new(mediator);
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
    mediator.shutdown().await.expect("actor worker shuts down");
}
