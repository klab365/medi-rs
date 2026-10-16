#![cfg(any(feature = "tokio", feature = "wasm", feature = "embassy"))]

use medi_rs::{medi_module, mediator};

#[allow(dead_code)]
struct ButtonModule {
    state: String,
}

medi_module! {
    manifest button_manifest;
    module ButtonModule;
    mailbox_capacity: 2;
}

mediator! {
    struct ActorMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [button_manifest];
    }
}

#[cfg(any(feature = "tokio", feature = "wasm"))]
struct ZeroCapacityModule;

#[cfg(any(feature = "tokio", feature = "wasm"))]
medi_module! {
    manifest zero_capacity_manifest;
    module ZeroCapacityModule;
    mailbox_capacity: 0;
}

#[cfg(any(feature = "tokio", feature = "wasm"))]
mediator! {
    struct ZeroCapacityMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [zero_capacity_manifest];
    }
}

#[test]
fn builder_accepts_and_stores_a_non_clone_module() {
    let _mediator = ActorMediator::builder()
        .button_module(ButtonModule {
            state: "ready".to_owned(),
        })
        .build();
}

#[cfg(any(feature = "tokio", feature = "wasm"))]
#[test]
fn try_build_rejects_zero_actor_mailbox_capacity() {
    assert!(matches!(
        ZeroCapacityMediator::builder()
            .zero_capacity_module(ZeroCapacityModule)
            .try_build(),
        Err(medi_rs::BuildError::InvalidMailboxCapacity {
            module: "ZeroCapacityModule"
        })
    ));
}
