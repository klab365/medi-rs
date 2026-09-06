#![cfg(feature = "tokio")]

use medi_rs::{Result, medi_handler, medi_module, medi_task, mediator};
use tokio::sync::Notify;

static TASK_EVENT_HANDLED: Notify = Notify::const_new();

#[derive(Clone)]
struct TaskStarted;

#[medi_task]
async fn publish_task_started(mediator: &TaskEventMediator) {
    mediator.publish(TaskStarted).await.unwrap();
}

#[medi_handler]
async fn observe_task_started(_: TaskStarted) -> Result<()> {
    TASK_EVENT_HANDLED.notify_one();
    Ok(())
}

medi_module! {
    manifest task_event_manifest;
    tasks { publish_task_started; }
    events { TaskStarted => [observe_task_started]; }
}

mediator! {
    struct TaskEventMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [task_event_manifest];
    }
}

#[tokio::test]
async fn tasks_can_publish_events_after_startup() {
    let mediator = Box::leak(Box::new(TaskEventMediator::new()));
    let handled = TASK_EVENT_HANDLED.notified();
    mediator.start();

    tokio::time::timeout(std::time::Duration::from_secs(1), handled)
        .await
        .expect("task-published event should be handled");
}
