#![cfg(feature = "tokio")]

use std::sync::atomic::{AtomicU32, Ordering};

use medi_rs::{ShutdownSignal, StartError, medi_module, medi_task, mediator};

#[derive(Clone, Copy)]
struct TaskState(&'static AtomicU32);

#[derive(Clone, Copy)]
struct TaskWithoutMediatorState(&'static AtomicU32);

#[derive(Clone, Copy)]
struct StartOnceState(&'static AtomicU32);

static TASK_STARTED: AtomicU32 = AtomicU32::new(0);
static TASK_WITHOUT_MEDIATOR_STARTED: AtomicU32 = AtomicU32::new(0);
static TASK_STOPPED: AtomicU32 = AtomicU32::new(0);
static START_ONCE: AtomicU32 = AtomicU32::new(0);

#[medi_task]
async fn initialize(_mediator: &RuntimeTaskMediator, state: TaskState) {
    state.0.store(1, Ordering::Release);
}

#[medi_task]
async fn initialize_without_mediator(state: TaskWithoutMediatorState) {
    state.0.store(1, Ordering::Release);
}

#[medi_task]
async fn wait_for_shutdown(signal: &ShutdownSignal) {
    signal.cancelled().await;
    TASK_STOPPED.store(1, Ordering::Release);
}

medi_module! {
    manifest runtime_task_manifest;
    resources { TaskState; TaskWithoutMediatorState; }
    tasks { initialize; initialize_without_mediator; wait_for_shutdown; }
}

mediator! {
    struct RuntimeTaskMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [runtime_task_manifest];
    }
}

mod start_once {
    use super::*;

    #[medi_task]
    async fn count_start(state: StartOnceState) {
        state.0.fetch_add(1, Ordering::AcqRel);
    }

    medi_module! {
        manifest start_once_manifest;
        resources { StartOnceState; }
        tasks { count_start; }
    }

    mediator! {
        pub struct StartOnceMediator {
            event_queue_capacity: 1;
            event_workers: 1;
            modules: [start_once_manifest];
        }
    }
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_starts_registered_tasks_with_resources() {
    TASK_STARTED.store(0, Ordering::Release);
    TASK_WITHOUT_MEDIATOR_STARTED.store(0, Ordering::Release);
    let mediator = Box::leak(Box::new(RuntimeTaskMediator::new(
        TaskState(&TASK_STARTED),
        TaskWithoutMediatorState(&TASK_WITHOUT_MEDIATOR_STARTED),
    )));
    mediator.start().expect("mediator must start");

    tokio::task::yield_now().await;
    assert_eq!(TASK_STARTED.load(Ordering::Acquire), 1);
    assert_eq!(TASK_WITHOUT_MEDIATOR_STARTED.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn tokio_start_is_once_only() {
    START_ONCE.store(0, Ordering::Release);
    let mediator = Box::leak(Box::new(start_once::StartOnceMediator::new(StartOnceState(
        &START_ONCE,
    ))));

    assert!(!mediator.is_started());
    assert_eq!(mediator.start(), Ok(()));
    assert!(mediator.is_started());
    assert_eq!(mediator.start(), Err(StartError::AlreadyStarted));

    tokio::task::yield_now().await;
    assert_eq!(START_ONCE.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn shutdown_signals_runtime_tasks() {
    TASK_STOPPED.store(0, Ordering::Release);
    let mediator = Box::leak(Box::new(RuntimeTaskMediator::new(
        TaskState(&TASK_STARTED),
        TaskWithoutMediatorState(&TASK_WITHOUT_MEDIATOR_STARTED),
    )));
    mediator.start().expect("mediator must start");
    mediator.shutdown().await.unwrap();
    assert_eq!(TASK_STOPPED.load(Ordering::Acquire), 1);
}
