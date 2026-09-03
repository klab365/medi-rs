#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppState {
    list: Arc<Mutex<Vec<String>>>,
}
impl AppState {
    fn new() -> Self {
        Self {
            list: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct Ping(String);

#[medi_handler]
async fn print_ping(state: AppState, req: Ping) -> Result<()> {
    state.list.lock().unwrap().push(req.0);
    Ok(())
}

medi_module! {
    manifest resource_manifest;
    resources { AppState; }
    commands { Ping => print_ping; }
}

mediator! {
    pub struct ResourceMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [resource_manifest];
    }
}

#[tokio::test]
async fn send_should_return_correct_value_from_the_resource() {
    let state = AppState::new();
    let mediator = ResourceMediator::new((state.clone(),));
    mediator.send(Ping("hello".into())).await.unwrap();
    mediator.send(Ping("world".into())).await.unwrap();
    assert_eq!(*state.list.lock().unwrap(), vec!["hello", "world"]);
}
