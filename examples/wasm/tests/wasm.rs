#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

#[wasm_bindgen_test(async)]
async fn greet_returns_message() {
    let greeting = medi_rs_wasm_example::greet("Rust".into()).await.unwrap();

    assert_eq!(greeting, "Hello, Rust!");
}

#[wasm_bindgen_test(async)]
async fn generated_mediator_greet_returns_message() {
    let greeting = medi_rs_wasm_example::greet_with_generated_mediator("Macro".into())
        .await
        .unwrap();

    assert_eq!(greeting, "Hello, Macro!");
}

#[wasm_bindgen_test(async)]
async fn publish_user_registered_returns_ok() {
    medi_rs_wasm_example::publish_user_registered("user@example.com".into())
        .await
        .unwrap();
}

#[wasm_bindgen_test(async)]
async fn generated_mediator_publish_returns_ok() {
    medi_rs_wasm_example::publish_with_generated_mediator("hello from test".into())
        .await
        .unwrap();
}
