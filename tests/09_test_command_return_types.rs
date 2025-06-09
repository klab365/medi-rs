use medi_rs::{BusBuilder, IntoCommand, Result};
use medi_rs_macros::MediCommand;

#[tokio::test]
async fn test_command_with_unit_return() {
    let bus = BusBuilder::default()
        .add_req_handler(handle_create_user)
        .build()
        .unwrap();

    let result = bus.send(CreateUserCommand { _name: "John".into() }).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_command_with_string_return() {
    let bus = BusBuilder::default()
        .add_req_handler(handle_get_greeting)
        .build()
        .unwrap();

    let result = bus.send(GetGreetingCommand { name: "John".into() }).await.unwrap();
    assert_eq!(result, "Hello, John!");
}

#[tokio::test]
async fn test_command_with_complex_return() {
    let bus = BusBuilder::default()
        .add_req_handler(handle_get_user_info)
        .build()
        .unwrap();

    let result = bus.send(GetUserInfoCommand { id: 42 }).await.unwrap();
    assert_eq!(result.id, 42);
    assert_eq!(result.name, "User 42");
    assert_eq!(result.age, 25);
}

// Command with default return type (unit)
#[derive(MediCommand)]
struct CreateUserCommand {
    _name: String,
}

// Command with String return type
#[derive(MediCommand)]
#[medi_command(return_type = String)]
struct GetGreetingCommand {
    name: String,
}

// Command with custom struct return type
#[derive(MediCommand)]
#[medi_command(return_type = UserInfo)]
struct GetUserInfoCommand {
    id: u32,
}

#[derive(Debug, PartialEq)]
struct UserInfo {
    id: u32,
    name: String,
    age: u32,
}

// Handlers
async fn handle_create_user(_req: CreateUserCommand) -> Result<()> {
    // Simulate creating a user
    println!("User created successfully");
    Ok(())
}

async fn handle_get_greeting(req: GetGreetingCommand) -> Result<String> {
    Ok(format!("Hello, {}!", req.name))
}

async fn handle_get_user_info(req: GetUserInfoCommand) -> Result<UserInfo> {
    Ok(UserInfo {
        id: req.id,
        name: format!("User {}", req.id),
        age: 25,
    })
}
