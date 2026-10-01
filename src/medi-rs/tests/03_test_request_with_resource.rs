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

struct NonCloneState {
    list: Mutex<Vec<String>>,
}

#[derive(MediCommand)]
#[medi_command(return_type = usize, error_type = medi_rs::Error)]
struct BorrowedPing(String);

#[medi_handler]
async fn print_borrowed_ping(
    _mediator: &BorrowedResourceMediator,
    state: &NonCloneState,
    req: BorrowedPing,
) -> Result<usize> {
    let mut list = state.list.lock().unwrap();
    list.push(req.0);
    Ok(list.len())
}

medi_module! {
    manifest resource_manifest;
    resources { app_state: AppState; }
    commands { Ping => print_ping; }
}

mediator! {
    pub struct ResourceMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [resource_manifest];
    }
}

medi_module! {
    manifest borrowed_resource_manifest;
    resources { state: NonCloneState; }
    commands { BorrowedPing => print_borrowed_ping; }
}

mediator! {
    pub struct BorrowedResourceMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [borrowed_resource_manifest];
    }
}

#[tokio::test]
async fn send_should_return_correct_value_from_the_resource() {
    let state = AppState::new();
    let mediator = ResourceMediator::builder().app_state(state.clone()).build();
    mediator.send(Ping("hello".into())).await.unwrap();
    mediator.send(Ping("world".into())).await.unwrap();
    assert_eq!(*state.list.lock().unwrap(), vec!["hello", "world"]);
}

#[tokio::test]
async fn send_should_borrow_a_non_clone_resource() {
    let mediator = BorrowedResourceMediator::builder()
        .state(NonCloneState {
            list: Mutex::new(Vec::new()),
        })
        .build();

    assert_eq!(mediator.send(BorrowedPing("hello".into())).await.unwrap(), 1);
    assert_eq!(mediator.send(BorrowedPing("world".into())).await.unwrap(), 2);
}
