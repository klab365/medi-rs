use medi_rs::error::Result;
use medi_rs::resource::from_resources::FromResources;
use medi_rs::traits::IntoReq;
use medi_rs::BusBuilder;

#[tokio::test]
async fn send_should_return_correct_value_from_the_resource() {
    let bus = BusBuilder::default()
        .add_req_handler(print_ping)
        .append_resources(AppState::new())
        .build();

    let pong = bus.send(Ping).await.unwrap();

    assert_eq!(pong, "Pong: hello");
}

#[derive(Debug, Clone)]
struct AppState {
    pub list: Vec<String>,
}
impl FromResources for AppState {}

impl AppState {
    pub fn new() -> Self {
        Self {
            list: vec!["hello".into()],
        }
    }
}

fn print_ping(state1: AppState, _req: Ping) -> Result<String> {
    Ok(format!("Pong: {}", state1.list[0]))
}

struct Ping;
impl IntoReq<String> for Ping {}
