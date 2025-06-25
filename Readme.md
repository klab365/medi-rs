# medi-rs

A flexible and lightweight mediator library for Rust, inspired by Axum's handler function pattern. `medi-rs` provides a clean and effective approach to command and event handling in Rust applications, with dependency injection support and powerful derive macros.

## Overview

`medi-rs` helps organize complex Rust applications by defining a clear separation between request-response operations and event-driven tasks. Usage examples can be found in the `tests` folder.

## Features

### Handler Types

`medi-rs` supports two primary handler types:

- **Request-Response Handler**: Designed for commands and queries. This handler receives a request and returns a response, ensuring the caller waits for the operation to complete.
  
- **Event Handler**: Tailored for events. It receives a request but does not return a response or make the caller wait. Instead, it publishes the event to all designated handlers, allowing for efficient, non-blocking event processing.

### Derive Macros

`medi-rs` provides convenient derive macros to reduce boilerplate code:

#### `#[derive(MediCommand)]`

Automatically implements the `IntoCommand` trait for command and query types. Supports specifying return types via attributes.

```rust
use medi_rs_macros::MediCommand;

// Command with default unit return type
#[derive(MediCommand)]
struct CreateUser {
    name: String,
}

// Command with String return type
#[derive(MediCommand)]
#[medi_command(return_type = String)]
struct GetGreeting {
    name: String,
}

// Command with custom struct return type
#[derive(MediCommand)]
#[medi_command(return_type = UserInfo)]
struct GetUserInfo {
    id: u32,
}

struct UserInfo {
    id: u32,
    name: String,
    email: String,
}
```

#### `#[derive(MediEvent)]`

Automatically implements the `IntoEvent` trait for event types.

```rust
use medi_rs_macros::MediEvent;

#[derive(Clone, MediEvent)]
struct UserCreated {
    name: String,
    email: String,
}

#[derive(Clone, MediEvent)]
struct OrderProcessed {
    order_id: u64,
    total_amount: f64,
}
```

#### `#[derive(MediRessource)]`

Automatically implements the `FromResources` trait for dependency injection. Works with both simple and generic structs.

```rust
use medi_rs_macros::MediRessource;
use std::sync::Arc;

#[derive(Clone, MediRessource)]
struct DatabaseConnection {
    connection_string: String,
}

// Also works with generic structs
#[derive(Clone, MediRessource)]
struct Repository<T: UserRepository> {
    inner: Arc<T>,
}
```

### Dependency Injection for Handlers

Handlers in `medi-rs` can be equipped with dependencies, simplifying access to shared resources. Use the `#[derive(MediRessource)]` macro to declare a struct as a dependency that can then be injected into handler functions. The maximum number of dependencies is 7.

#### Complete Example

```rust
use medi_rs::{BusBuilder, Result};
use medi_rs_macros::{MediCommand, MediRessource};
use std::sync::{Arc, Mutex};

#[derive(Clone, MediRessource)]
struct UserRepository {
    users: Arc<Mutex<Vec<User>>>,
}

impl UserRepository {
    fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn save(&self, user: User) -> Result<()> {
        self.users.lock().unwrap().push(user);
        Ok(())
    }
}

#[derive(MediCommand)]
struct CreateUser {
    name: String,
}

struct User {
    name: String,
}

async fn handle_create_user(repo: UserRepository, req: CreateUser) -> Result<()> {
    let user = User { name: req.name };
    repo.save(user)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let repo = UserRepository::new();
    let bus = BusBuilder::default()
        .add_req_handler(handle_create_user)
        .append_resources(repo)
        .build()?;

    bus.send(CreateUser { name: "John".into() }).await?;
    Ok(())
}
```

### Event Handling Example

```rust
use medi_rs::{Bus, Result};
use medi_rs_macros::{MediEvent, MediRessource};
use std::sync::{Arc, Mutex};

#[derive(Clone, MediEvent)]
struct UserRegistered {
    user_id: u64,
    email: String,
}

#[derive(Clone, MediRessource)]
struct EmailService {
    sent_emails: Arc<Mutex<Vec<String>>>,
}

async fn send_welcome_email(
    email_service: EmailService,
    event: UserRegistered,
) -> Result<()> {
    // Send welcome email logic
    let email = format!("Welcome email sent to {}", event.email);
    email_service.sent_emails.lock().unwrap().push(email);
    Ok(())
}

async fn update_analytics(event: UserRegistered) -> Result<()> {
    // Update analytics logic
    println!("User {} registered in analytics", event.user_id);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let email_service = EmailService {
        sent_emails: Arc::new(Mutex::new(Vec::new())),
    };
    
    let bus = Bus::builder()
        .add_event_handler(send_welcome_email)
        .add_event_handler(update_analytics)
        .append_resources(email_service)
        .build()?;

    // Publish event - both handlers will be called
    bus.publish(UserRegistered {
        user_id: 123,
        email: "user@example.com".into(),
    }).await?;
    
    Ok(())
}
```

## Getting Started

All commands for building, testing, and running the project are defined in the `Justfile` and can be executed with the just command.

### Development Environment Setup

You can quickly set up the development environment using the provided DevContainer configuration. Prerequisites are Docker and Visual Studio Code with the Remote - Containers extension.

1. Open the project in Visual Studio Code.
2. When prompted, select Reopen in Container to initialize the environment.

### Building the Project

To build the project, run the following command:

```sh
just build
```

### Running Tests

To run the test suite, execute the following command:

```sh
just test
```
