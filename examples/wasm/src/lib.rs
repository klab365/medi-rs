use gloo_timers::future::TimeoutFuture;
use medi_rs::{MediCommand, Result, medi_handler, medi_module, medi_task, mediator};
use std::{cell::RefCell, thread_local};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(message: &str);
}

#[derive(MediCommand)]
#[medi_command(return_type = String, error_type = medi_rs::Error)]
struct Greet {
    name: String,
}

#[derive(Clone)]
struct UserRegistered {
    email: String,
}

#[derive(Clone)]
struct AuditEvent {
    message: String,
}

#[medi_handler]
async fn greet_handler(request: Greet) -> Result<String> {
    Ok(format!("Hello, {}!", request.name))
}

#[medi_handler]
async fn user_registered_handler(event: UserRegistered) -> Result<()> {
    log(&format!("welcome email queued for {}", event.email));
    Ok(())
}

#[medi_handler]
async fn audit_event_handler(event: AuditEvent) -> Result<()> {
    log(&format!("audit: {}", event.message));
    Ok(())
}

#[medi_task]
async fn announce_startup(_mediator: &WasmMediator) {
    for i in 0..10 {
        log(&format!("medi-rs WASM runtime task is running: {} seconds elapsed", i));
        TimeoutFuture::new(1_000).await;
    }
}

medi_module! {
    manifest greeter_manifest;
    commands { Greet => greet_handler; }
}

medi_module! {
    manifest email_manifest;
    events { UserRegistered => [user_registered_handler]; }
}

medi_module! {
    manifest audit_manifest;
    events { AuditEvent => [audit_event_handler]; }
}

medi_module! {
    manifest runtime_task_manifest;
    tasks { announce_startup; }
}

mediator! {
    pub struct WasmMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [greeter_manifest, email_manifest, audit_manifest, runtime_task_manifest];
    }
}

thread_local! {
    static MEDIATOR: RefCell<Option<&'static WasmMediator>> = const { RefCell::new(None) };
}

/// Send a request/response command through the generated WASM mediator.
#[wasm_bindgen]
pub async fn greet(name: String) -> core::result::Result<String, JsValue> {
    mediator().send(Greet { name }).await.map_err(to_js_error)
}

/// Publish an event through the generated WASM mediator.
#[wasm_bindgen]
pub async fn publish_user_registered(email: String) -> core::result::Result<(), JsValue> {
    mediator().publish(UserRegistered { email }).await.map_err(to_js_error)
}

/// Send a command through the same generated mediator used by the other exports.
#[wasm_bindgen]
pub async fn greet_with_generated_mediator(name: String) -> core::result::Result<String, JsValue> {
    greet(name).await
}

/// Publish an event through the same generated mediator used by the other exports.
#[wasm_bindgen]
pub async fn publish_with_generated_mediator(message: String) -> core::result::Result<(), JsValue> {
    mediator().publish(AuditEvent { message }).await.map_err(to_js_error)
}

fn mediator() -> &'static WasmMediator {
    MEDIATOR.with(|cell| {
        if let Some(mediator) = *cell.borrow() {
            return mediator;
        }

        let mediator = Box::leak(Box::new(WasmMediator::new()));
        mediator.start();
        *cell.borrow_mut() = Some(mediator);
        mediator
    })
}

fn to_js_error(error: medi_rs::Error) -> JsValue {
    JsValue::from_str(&format!("{error:?}"))
}
