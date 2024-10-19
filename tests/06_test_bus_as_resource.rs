use medi_rs::{Bus, HandlerResult, IntoCommand};

#[tokio::test]
async fn send_call_second_req_test() {
    let bus = Bus::builder()
        .add_req_handler(validate_user)
        .add_req_handler(create_user_dyn)
        .build()
        .unwrap();

    bus.send(CreateUser { name: "hello".into() }).await.unwrap();
}

struct CreateUser {
    name: String,
}
impl IntoCommand<()> for CreateUser {}

struct ValidateUser {
    name: String,
}
impl IntoCommand<()> for ValidateUser {}

// handler functions...

async fn create_user_dyn(bus: Bus, req: CreateUser) -> HandlerResult<()> {
    println!("Creating user: {}", req.name);
    bus.send(ValidateUser { name: req.name }).await.unwrap();
    Ok(())
}

async fn validate_user(req: ValidateUser) -> HandlerResult<()> {
    println!("Validating user: {}", req.name);
    Ok(())
}
