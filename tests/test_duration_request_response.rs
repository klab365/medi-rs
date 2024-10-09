use medi_rs::{traits::IntoReq, BusBuilder};
use medi_rs::error::Result;
use serde::{Deserialize, Serialize};

#[tokio::test]
async fn send_should_take_less_than_1ms() {
    let bus = BusBuilder::default().add_req_handler(print_ping).build();

    let watch = std::time::Instant::now();
    let pong = bus.send(Ping("Ping".to_string())).await.unwrap();
    let duration = watch.elapsed();

    assert_eq!(pong, "Pong: Ping");
    assert!(duration.as_millis() < 1);
}

fn print_ping(ping: Ping) -> Result<String> {
    Ok(format!("Pong: {}", ping.0))
}

#[derive(Serialize, Deserialize)]
struct Ping(String);
impl IntoReq<String> for Ping {}