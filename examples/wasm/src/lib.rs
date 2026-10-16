use core::convert::Infallible;
use medi_rs::{MediCommand, MediContext, medi_module, mediator};
use std::{cell::RefCell, thread_local};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(message: &str);
}

#[derive(MediCommand)]
#[medi_command(return_type = String, error_type = Infallible)]
struct Greet {
    name: String,
}

#[derive(Clone)]
struct UserRegistered {
    email: String,
}

struct Greeter;

impl Greeter {
    async fn greet(
        &mut self,
        _context: &MediContext<'_, WasmMediator>,
        request: Greet,
    ) -> Result<String, Infallible> {
        Ok(format!("Hello, {}!", request.name))
    }

    async fn user_registered(
        &mut self,
        _context: &MediContext<'_, WasmMediator>,
        event: UserRegistered,
    ) -> Result<(), Infallible> {
        log(&format!("welcome email queued for {}", event.email));
        Ok(())
    }
}

medi_module! {
    manifest greeter_manifest;
    module Greeter;
    mailbox_capacity: 8;
    commands { Greet => Greeter::greet; }
    events { UserRegistered => [Greeter::user_registered]; }
}

mediator! {
    pub struct WasmMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [greeter_manifest];
    }
}

thread_local! {
    static MEDIATOR: RefCell<Option<&'static WasmMediator>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub async fn greet(name: String) -> Result<String, JsValue> {
    mediator().send(Greet { name }).await.map_err(to_js_error)
}

#[wasm_bindgen]
pub async fn publish_user_registered(email: String) -> Result<(), JsValue> {
    mediator().publish(UserRegistered { email }).await.map_err(to_js_error)
}

fn mediator() -> &'static WasmMediator {
    MEDIATOR.with(|cell| {
        if let Some(mediator) = *cell.borrow() {
            return mediator;
        }
        let mediator = Box::leak(Box::new(WasmMediator::builder().greeter(Greeter).build()));
        mediator.start().expect("actor worker starts");
        *cell.borrow_mut() = Some(mediator);
        mediator
    })
}

fn to_js_error(error: impl core::fmt::Debug) -> JsValue {
    JsValue::from_str(&format!("{error:?}"))
}
