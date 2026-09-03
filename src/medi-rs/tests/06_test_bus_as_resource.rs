#![cfg(feature = "tokio")]

use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct CreateUser {
    name: String,
}

#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct ValidateUser {
    name: String,
}

#[medi_handler]
async fn create_user(mediator: &UserMediator, request: CreateUser) -> Result<()> {
    mediator.send(ValidateUser { name: request.name }).await?;
    Ok(())
}

#[medi_handler]
async fn validate_user(request: ValidateUser) -> Result<()> {
    assert!(!request.name.is_empty());
    Ok(())
}

medi_module! {
    manifest users_manifest;
    commands {
        CreateUser => create_user;
        ValidateUser => validate_user;
    }
}

mediator! {
    pub struct UserMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [users_manifest];
    }
}

#[tokio::test]
async fn send_call_second_req_test() {
    UserMediator::new()
        .send(CreateUser { name: "hello".into() })
        .await
        .unwrap();
}
