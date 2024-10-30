# medi-rs

A flexible and lightweight mediator library for Rust, inspired by Axum’s handler function pattern. `medi-rs` provides a clean and effective approach to command and event handling in Rust applications, with dependency injection support.

## Overview

`medi-rs` helps organize complex Rust applications by defining a clear separation between request-response operations and event-driven tasks. Usage examples can be found in the `tests` folder.

## Features

### Handler Types

`medi-rs` supports two primary handler types:

- **Request-Response Handler**: Designed for commands and queries. This handler receives a request and returns a response, ensuring the caller waits for the operation to complete.
  
- **Event Handler**: Tailored for events. It receives a request but does not return a response or make the caller wait. Instead, it publishes the event to all designated handlers, allowing for efficient, non-blocking event processing.

### Dependency Injection for Handlers

Handlers in `medi-rs` can be equipped with dependencies, simplifying access to shared resources. Use the `FromResource` trait to declare a struct as a dependency that can then be injected into handler functions. The maximum numerber of dependencies is 7.

#### Example

```rust
#[derive(Debug)]
struct MyResource;

impl FromResource for MyResource {}

async fn handler(dep1: MyResource, req: Request) -> HandlerResult<()> {
    // Handler logic here
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
