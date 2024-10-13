use medi_rs::{BusBuilder, HandlerResult, IntoReq};
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
            let rand_time = rand::random::<u64>() % 100;
            tokio::time::sleep(tokio::time::Duration::from_millis(rand_time)).await;
            let pong = bus.send(Ping(format!("Ping{}", i))).await.unwrap();
            println!("Pong: {}", pong.0);
            assert_eq!(pong.0, format!("Pong: Ping{}", i));
        });

        handlers.push(handler);
    }

    for handler in handlers {
        handler.await.unwrap();
    }
}

async fn print_ping(id: Ping) -> HandlerResult<Pong> {
    Ok(Pong(format!("Pong: {}", id.0)))
}

struct Ping(String);
impl IntoReq<Pong> for Ping {}

#[derive(Debug)]
struct Pong(String);