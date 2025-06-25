use medi_rs::BusBuilder;
use medi_rs::FromResources;
use medi_rs::{IntoCommand, Result};
use medi_rs_macros::MediCommand;
use medi_rs_macros::MediRessource;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn send_should_return_correct_value_from_the_resource() {
    let state = AppState::new();
    let bus = BusBuilder::default()
        .add_req_handler(print_ping)
        .append_resources(state.clone())
        .build()
        .unwrap();

    bus.send(Ping("hello".into())).await.unwrap();
    bus.send(Ping("world".into())).await.unwrap();

    let state = state.list.lock().unwrap();
    assert_eq!(state.len(), 2);
    assert_eq!(state[0], "hello");
    assert_eq!(state[1], "world");
}

#[derive(Clone, MediRessource)]
struct AppState {
    pub list: Arc<Mutex<Vec<String>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            list: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

async fn print_ping(state: AppState, req: Ping) -> Result<()> {
    state.list.lock().unwrap().push(req.0);
    Ok(())
}

#[derive(MediCommand)]
struct Ping(String);
