#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
use std::sync::Arc;

#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct CreateUser;
#[derive(MediCommand)]
#[medi_command(return_type = String, error_type = medi_rs::Error)]
struct Greeting;
#[derive(MediCommand)]
#[medi_command(return_type = UserInfo, error_type = medi_rs::Error)]
struct UserInfoRequest;
#[derive(MediCommand)]
#[medi_command(return_type = Arc<UserInfo>, error_type = medi_rs::Error)]
struct SharedUserInfoRequest;
#[derive(Debug, PartialEq)]
struct UserInfo {
    id: u32,
    name: String,
}

#[medi_handler]
async fn create(_: CreateUser) -> Result<()> {
    Ok(())
}
#[medi_handler]
async fn greeting(_: Greeting) -> Result<String> {
    Ok("Hello".into())
}
#[medi_handler]
async fn user(_: UserInfoRequest) -> Result<UserInfo> {
    Ok(UserInfo {
        id: 42,
        name: "Ada".into(),
    })
}
#[medi_handler]
async fn shared(_: SharedUserInfoRequest) -> Result<Arc<UserInfo>> {
    Ok(Arc::new(UserInfo {
        id: 7,
        name: "Grace".into(),
    }))
}

medi_module! { manifest return_manifest; commands { CreateUser => create; Greeting => greeting; UserInfoRequest => user; SharedUserInfoRequest => shared; } }
mediator! { pub struct ReturnMediator { event_queue_capacity: 1; event_workers: 1; modules: [return_manifest]; } }

#[tokio::test]
async fn command_return_types_are_preserved() {
    let mediator = ReturnMediator::new();
    mediator.send(CreateUser).await.unwrap();
    assert_eq!(mediator.send(Greeting).await.unwrap(), "Hello");
    assert_eq!(mediator.send(UserInfoRequest).await.unwrap().id, 42);
    assert_eq!(mediator.send(SharedUserInfoRequest).await.unwrap().name, "Grace");
}
