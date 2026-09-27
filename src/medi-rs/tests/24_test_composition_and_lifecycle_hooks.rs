#![cfg(feature = "tokio")]

use std::sync::atomic::{AtomicUsize, Ordering};

use medi_rs::{MediCommand, RouteKind, medi_handler, medi_module, medi_shutdown, medi_startup, mediator};

#[derive(Clone, Copy)]
struct HookState(&'static AtomicUsize);

static HOOK_STATE: AtomicUsize = AtomicUsize::new(0);

#[medi_startup]
fn initialize(_mediator: &LifecycleMediator, state: &HookState) {
    assert_eq!(state.0.fetch_add(1, Ordering::AcqRel), 0);
}

#[medi_shutdown]
fn cleanup(_mediator: &LifecycleMediator, state: &HookState) {
    assert_eq!(state.0.fetch_add(1, Ordering::AcqRel), 1);
}

#[derive(MediCommand)]
struct InspectCommand;

#[medi_handler]
async fn inspect_command(_: InspectCommand) -> Result<(), core::convert::Infallible> {
    Ok(())
}

medi_module! {
    manifest lifecycle_manifest;
    resources { HookState; }
    commands { InspectCommand => inspect_command; }
    startup { initialize; }
    shutdown { cleanup; }
}

mediator! {
    struct LifecycleMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [lifecycle_manifest];
    }
}

#[tokio::test]
async fn composition_is_static_and_describes_routes() {
    let description = LifecycleMediator::composition();
    assert_eq!(description.mediator_name, "LifecycleMediator");
    assert_eq!(description.module_count, 1);
    assert_eq!(description.resource_names, ["HookState"]);
    assert_eq!(description.routes.len(), 1);
    assert_eq!(description.routes[0].kind, RouteKind::Command);
    assert_eq!(description.routes[0].message_name, "InspectCommand");
    assert_eq!(description.routes[0].handler_names, ["inspect_command"]);

    LifecycleMediator::new(HookState(&HOOK_STATE))
        .send(InspectCommand)
        .await
        .unwrap();
}

#[tokio::test]
async fn lifecycle_hooks_run_once_in_startup_then_shutdown_order() {
    HOOK_STATE.store(0, Ordering::Release);
    let mediator = Box::leak(Box::new(LifecycleMediator::new(HookState(&HOOK_STATE))));

    mediator.start().unwrap();
    assert_eq!(HOOK_STATE.load(Ordering::Acquire), 1);
    mediator.shutdown().await.unwrap();
    assert_eq!(HOOK_STATE.load(Ordering::Acquire), 2);
}
