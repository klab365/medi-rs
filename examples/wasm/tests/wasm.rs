#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

#[wasm_bindgen_test(async)]
async fn actor_command_returns_a_reply() {
    let greeting = medi_rs_wasm_example::greet("Rust".into()).await.unwrap();
    assert_eq!(greeting, "Hello, Rust!");
}

#[wasm_bindgen_test(async)]
async fn actor_event_is_published() {
    medi_rs_wasm_example::publish_user_registered("user@example.com".into())
        .await
        .unwrap();
}
