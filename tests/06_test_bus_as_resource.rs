use medi_rs::{Bus, IntoCommand, Result};
use medi_rs_macros::MediCommand;

#[tokio::test]
async fn send_call_second_req_test() {
    let bus = Bus::builder()
        .add_req_handler(validate_user)
        .add_req_handler(create_user_dyn)
        .build()
        .unwrap();

    let res = bus.send(CreateUser { name: "hello".into() }).await;

    assert!(res.is_ok());
}

#[derive(MediCommand)]
struct CreateUser {
    name: String,
}

#[derive(MediCommand)]
struct ValidateUser {
    name: String,
}

// handler functions...
async fn create_user_dyn(bus: Bus, req: CreateUser) -> Result<()> {
    println!("Creating user: {}", req.name);
    bus.send(ValidateUser { name: req.name }).await?;
    Ok(())
}

async fn validate_user(req: ValidateUser) -> Result<()> {
    println!("Validating user: {}", req.name);
    Ok(())
}
