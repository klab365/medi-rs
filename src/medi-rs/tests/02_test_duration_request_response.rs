#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
#[medi_command(return_type = String, error_type = medi_rs::Error)]
struct Ping(String);

#[medi_handler]
async fn print_ping(ping: Ping) -> Result<String> {
    Ok(format!("Pong: {}", ping.0))
}

medi_module! {
    manifest latency_manifest;
    commands { Ping => print_ping; }
}

mediator! {
    pub struct LatencyMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [latency_manifest];
    }
}

#[tokio::test]
async fn send_should_take_less_than_1ms() {
    let mediator = LatencyMediator::new();
    let watch = std::time::Instant::now();
    let pong = mediator.send(Ping("Ping".into())).await.unwrap();
    assert_eq!(pong, "Pong: Ping");
    assert!(watch.elapsed().as_millis() < 1);
}
