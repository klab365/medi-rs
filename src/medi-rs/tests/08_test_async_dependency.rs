#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
use std::sync::{Arc, Mutex};

#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct CreateUser {
    name: String,
}
#[derive(Clone)]
struct UserCreated {
    name: String,
}
struct User {
    name: String,
}

#[derive(Clone)]
struct State(Arc<Mutex<Vec<User>>>);

#[medi_handler]
async fn create_user(mediator: &AsyncMediator, state: State, request: CreateUser) -> Result<()> {
    state.0.lock().unwrap().push(User {
        name: request.name.clone(),
    });
    mediator.publish(UserCreated { name: request.name }).await
}
#[medi_handler]
async fn user_created(state: State, event: UserCreated) -> Result<()> {
    state.0.lock().unwrap().push(User { name: event.name });
    Ok(())
}

medi_module! {
    manifest async_manifest;
    resources { State; }
    commands { CreateUser => create_user; }
    events { UserCreated => [user_created]; }
}
mediator! {
    pub struct AsyncMediator {
        event_queue_capacity: 4;
        event_workers: 1;
        modules: [async_manifest];
    }
}

#[tokio::test]
async fn send_should_work_with_async_dependency_and_event() {
    let state = State(Arc::new(Mutex::new(Vec::new())));
    let mediator = Box::leak(Box::new(AsyncMediator::new((state.clone(),))));
    mediator.start();
    mediator.send(CreateUser { name: "John".into() }).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    let names: Vec<_> = state.0.lock().unwrap().iter().map(|user| user.name.clone()).collect();
    assert_eq!(names, ["John", "John"]);
}
