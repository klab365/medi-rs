use medi_rs::BusBuilder;
use medi_rs::FromResources;
use medi_rs::{HandlerResult, IntoReq};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn send_should_return_correct_value_from_the_resource() {
    let state = AppState::new();
    let bus = BusBuilder::default()
        .add_req_handler(print_ping)
        .append_resources(state.clone())
        .build();

    bus.send(Ping("hello".into())).await.unwrap();
    bus.send(Ping("world".into())).await.unwrap();

    let state = state.list.lock().unwrap();
    assert_eq!(state.len(), 2);
    assert_eq!(state[0], "hello");
    assert_eq!(state[1], "world");
}

#[derive(Clone)]
struct AppState {
    pub list: Arc<Mutex<Vec<String>>>,
}
impl FromResources for AppState {}

impl AppState {
    pub fn new() -> Self {
        Self {
            list: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

async fn print_ping(state: AppState, req: Ping) -> HandlerResult<()> {
    state.list.lock().unwrap().push(req.0);
    Ok(())
}

struct Ping(String);
impl IntoReq<()> for Ping {}
