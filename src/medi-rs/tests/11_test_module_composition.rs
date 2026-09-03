#![cfg(feature = "tokio")]

use core::convert::Infallible;
use core::sync::atomic::{AtomicUsize, Ordering};

use medi_rs::{medi_handler, medi_module, mediator};
use medi_rs_macros::MediCommand;

#[derive(MediCommand)]
struct CreateUser;

#[derive(MediCommand)]
struct RecordAudit;

#[allow(dead_code)]
#[derive(Clone)]
struct UserCreated;

#[derive(Clone)]
struct UserRepository(&'static str);

#[derive(Clone)]
struct AuditRepository(&'static str);

#[medi_handler]
async fn create_user(mediator: &AppMediator, repository: UserRepository, _: CreateUser) -> Result<(), Infallible> {
    let _ = mediator;
    assert_eq!(AppMediator::EVENT_QUEUE_CAPACITY, 16);
    assert_eq!(repository.0, "users");
    Ok(())
}

#[medi_handler]
async fn record_audit(repository: AuditRepository, _: RecordAudit) -> Result<(), Infallible> {
    assert_eq!(repository.0, "audit");
    Ok(())
}

static EVENT_HANDLERS_RUN: AtomicUsize = AtomicUsize::new(0);

#[medi_handler]
async fn send_welcome_email(_: UserCreated) -> Result<(), Infallible> {
    EVENT_HANDLERS_RUN.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

#[medi_handler]
async fn write_audit_log(_: UserCreated) -> Result<(), Infallible> {
    EVENT_HANDLERS_RUN.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

mod audit {
    use super::medi_module;

    medi_module! {
        manifest audit_manifest;
        commands { RecordAudit => record_audit; }
        events { UserCreated => [write_audit_log]; }
        resources { AuditRepository; }
    }
}

mod users {
    use super::medi_module;

    medi_module! {
        manifest users_manifest;
        commands { CreateUser => create_user; }
        events { UserCreated => [send_welcome_email]; }
        resources { UserRepository; }
    }
}

use audit::audit_manifest;
use users::users_manifest;

mediator! {
    pub struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [users_manifest, audit_manifest];
    }
}

#[test]
fn composition_collects_manifests_from_separate_modules() {
    assert_eq!(AppMediator::MODULE_COUNT, 2);
}

#[tokio::test]
async fn composition_generates_static_command_routes() {
    let mediator = AppMediator::new((UserRepository("users"), AuditRepository("audit")));

    mediator.send(CreateUser).await.unwrap();
    mediator.send(RecordAudit).await.unwrap();
}

#[tokio::test]
async fn composition_generates_static_event_routes() {
    EVENT_HANDLERS_RUN.store(0, Ordering::SeqCst);
    let mediator = Box::leak(Box::new(AppMediator::new((
        UserRepository("users"),
        AuditRepository("audit"),
    ))));
    mediator.start();

    mediator.publish(UserCreated).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    assert_eq!(EVENT_HANDLERS_RUN.load(Ordering::SeqCst), 2);
}
