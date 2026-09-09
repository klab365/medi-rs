#![cfg(feature = "tokio")]

use medi_rs::{EventFailureReporter, EventHandlerFailure, Result, StartError, medi_handler, medi_module, mediator};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone)]
struct BaseEvent;
#[derive(Clone, Default)]
struct InMemoryMsgQueue(Arc<Mutex<Vec<&'static str>>>);

#[medi_handler]
async fn handler_one(queue: InMemoryMsgQueue, _: BaseEvent) -> Result<()> {
    queue.0.lock().unwrap().push("one");
    Ok(())
}
#[medi_handler]
async fn handler_two(queue: InMemoryMsgQueue, _: BaseEvent) -> Result<()> {
    queue.0.lock().unwrap().push("two");
    Ok(())
}
#[medi_handler]
async fn handler_three(queue: InMemoryMsgQueue, _: BaseEvent) -> Result<()> {
    queue.0.lock().unwrap().push("three");
    Ok(())
}

medi_module! {
    manifest event_manifest;
    resources { InMemoryMsgQueue; }
    events { BaseEvent => [handler_one, handler_two, handler_three]; }
}
mediator! {
    pub struct EventMediator {
        event_queue_capacity: 8;
        event_workers: 1;
        modules: [event_manifest];
    }
}

mod failure_reporting {
    use super::*;

    static FAILURE_REPORTS: AtomicUsize = AtomicUsize::new(0);
    static FOLLOWING_HANDLERS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Clone)]
    struct FailingEvent;

    struct FailureReporter;

    impl EventFailureReporter for FailureReporter {
        async fn report(&self, failure: EventHandlerFailure) {
            assert_eq!(failure.event_name(), "FailingEvent");
            assert_eq!(failure.handler_name(), "fail_event");
            FAILURE_REPORTS.fetch_add(1, Ordering::Release);
        }
    }

    #[medi_handler]
    async fn fail_event(_: FailingEvent) -> core::result::Result<(), &'static str> {
        Err("expected failure")
    }

    #[medi_handler]
    async fn handle_after_failure(_: FailingEvent) -> core::result::Result<(), &'static str> {
        FOLLOWING_HANDLERS.fetch_add(1, Ordering::Release);
        Ok(())
    }

    medi_module! {
        manifest failure_manifest;
        events { FailingEvent => [fail_event, handle_after_failure]; }
    }

    mediator! {
        struct FailureMediator {
            event_queue_capacity: 1;
            event_workers: 1;
            modules: [failure_manifest];
            event_failure_reporter: FailureReporter;
        }
    }

    #[tokio::test]
    async fn event_failure_reporter_observes_failures_without_stopping_dispatch() {
        FAILURE_REPORTS.store(0, Ordering::Release);
        FOLLOWING_HANDLERS.store(0, Ordering::Release);
        let mediator = Box::leak(Box::new(FailureMediator::new()));
        mediator.start().expect("mediator must start");
        mediator.publish(FailingEvent).await.unwrap();

        tokio::time::timeout(tokio::time::Duration::from_secs(1), async {
            while FAILURE_REPORTS.load(Ordering::Acquire) != 1 || FOLLOWING_HANDLERS.load(Ordering::Acquire) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("failure must be reported and later handler must run");
    }
}

#[tokio::test]
async fn publish_should_process_published_event() {
    let queue = InMemoryMsgQueue::default();
    let mediator = Box::leak(Box::new(EventMediator::new(queue.clone())));
    mediator.start().expect("mediator must start");
    assert_eq!(mediator.start(), Err(StartError::AlreadyStarted));
    mediator.publish(BaseEvent).await.unwrap();
    mediator.publish(BaseEvent).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(queue.0.lock().unwrap().len(), 6);
}
