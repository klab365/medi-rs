#![cfg(feature = "tokio")]

use std::sync::atomic::{AtomicU32, Ordering};

use medi_rs::{medi_module, medi_task, mediator};

#[derive(Clone, Copy)]
struct TaskState(&'static AtomicU32);

static TASK_STARTED: AtomicU32 = AtomicU32::new(0);

#[medi_task]
async fn initialize(_mediator: &RuntimeTaskMediator, state: TaskState) {
    state.0.store(1, Ordering::Release);
}

medi_module! {
    manifest runtime_task_manifest;
    resources { TaskState; }
    tasks { initialize; }
}

mediator! {
    struct RuntimeTaskMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [runtime_task_manifest];
    }
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn tokio_starts_registered_tasks_with_resources() {
    TASK_STARTED.store(0, Ordering::Release);
    let mediator = Box::leak(Box::new(RuntimeTaskMediator::new((TaskState(&TASK_STARTED),))));
    mediator.start();

    tokio::task::yield_now().await;
    assert_eq!(TASK_STARTED.load(Ordering::Acquire), 1);
}
