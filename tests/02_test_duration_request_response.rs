use medi_rs::{BusBuilder, IntoCommand, Result};

#[tokio::test]
async fn send_should_take_less_than_1ms() {
    let bus = BusBuilder::default().add_req_handler(print_ping).build().unwrap();

    let watch = std::time::Instant::now();
    let pong = bus.send(Ping("Ping".to_string())).await.unwrap();
    let duration = watch.elapsed();

    println!("Duration: {:?}", duration);

    assert_eq!(pong, "Pong: Ping");
    assert!(duration.as_millis() < 1); // the call should take less than 1ms
}

async fn print_ping(ping: Ping) -> Result<String> {
    Ok(format!("Pong: {}", ping.0))
}

struct Ping(String);
impl IntoCommand<String> for Ping {}
