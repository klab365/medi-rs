#![cfg(feature = "tokio")]

use std::time::Duration;

use medi_rs::{Result, medi_handler, medi_module, mediator};

mod zero_capacity {
    use super::*;

    #[derive(Clone)]
    struct Event;
    #[allow(dead_code)]
    #[medi_handler]
    async fn discard(_: Event) -> Result<()> {
        Ok(())
    }
    medi_module! { manifest manifest; events { Event => [discard]; } }
    mediator! { struct Mediator { event_queue_capacity: 0; event_workers: 1; modules: [manifest]; } }

    #[test]
    #[should_panic(expected = "event_queue_capacity must be greater than zero")]
    fn constructor_rejects_zero_queue_capacity() {
        let _ = Mediator::new();
    }
}

mod zero_workers {
    use super::*;

    #[derive(Clone)]
    struct Event;
    #[allow(dead_code)]
    #[medi_handler]
    async fn discard(_: Event) -> Result<()> {
        Ok(())
    }
    const ZERO_WORKERS: usize = 0;

    medi_module! { manifest manifest; events { Event => [discard]; } }
    mediator! { struct Mediator { event_queue_capacity: 1; event_workers: ZERO_WORKERS; modules: [manifest]; } }

    #[test]
    #[should_panic(expected = "event_workers must be greater than zero")]
    fn start_rejects_zero_workers() {
        let mediator = Box::leak(Box::new(Mediator::new()));
        mediator.start();
    }
}

mod backpressure {
    use super::*;

    #[derive(Clone)]
    struct Event;
    #[allow(dead_code)]
    #[medi_handler]
    async fn discard(_: Event) -> Result<()> {
        Ok(())
    }
    medi_module! { manifest manifest; events { Event => [discard]; } }
    mediator! { struct Mediator { event_queue_capacity: 1; event_workers: 1; modules: [manifest]; } }

    #[test]
    fn try_publish_reports_a_full_queue_without_waiting() {
        let mediator = Mediator::new();
        assert!(mediator.try_publish(Event).is_ok());
        assert!(matches!(
            mediator.try_publish(Event),
            Err(medi_rs::TryPublishError::Full(Event))
        ));
    }

    #[tokio::test]
    async fn publish_waits_while_the_bounded_queue_is_full() {
        let mediator = Mediator::new();
        mediator.publish(Event).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), mediator.publish(Event))
                .await
                .is_err()
        );
    }
}

mod handler_errors {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Notify;

    #[derive(Clone)]
    struct Event;

    #[derive(Clone)]
    struct Completion(Arc<Notify>);

    #[medi_handler]
    async fn failing_handler(_: Event) -> Result<()> {
        Err(medi_rs::Error::EventProcessingError)
    }

    #[medi_handler]
    async fn succeeding_handler(completion: Completion, _: Event) -> Result<()> {
        completion.0.notify_one();
        Ok(())
    }

    medi_module! {
        manifest manifest;
        resources { Completion; }
        events { Event => [failing_handler, succeeding_handler]; }
    }

    mediator! {
        struct Mediator {
            event_queue_capacity: 1;
            event_workers: 1;
            modules: [manifest];
        }
    }

    #[tokio::test]
    async fn failures_do_not_prevent_later_handlers() {
        let completion = Completion(Arc::new(Notify::new()));
        let mediator = Box::leak(Box::new(Mediator::new(completion.clone())));
        mediator.start();

        let completed = completion.0.notified();
        mediator.publish(Event).await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), completed)
            .await
            .expect("later event handler should run despite an earlier error");
    }
}

mod multiple_workers {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone)]
    struct Event;

    #[derive(Clone)]
    struct Counter(Arc<AtomicUsize>);

    #[medi_handler]
    async fn count(counter: Counter, _: Event) -> Result<()> {
        counter.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    medi_module! {
        manifest manifest;
        resources { Counter; }
        events { Event => [count]; }
    }

    mediator! {
        struct Mediator {
            event_queue_capacity: 32;
            event_workers: 2;
            modules: [manifest];
        }
    }

    #[tokio::test]
    async fn dispatch_each_queued_event_once() {
        const EVENT_COUNT: usize = 20;

        let counter = Counter(Arc::new(AtomicUsize::new(0)));
        let mediator = Box::leak(Box::new(Mediator::new(counter.clone())));
        mediator.start();

        for _ in 0..EVENT_COUNT {
            mediator.publish(Event).await.unwrap();
        }

        tokio::time::timeout(Duration::from_secs(1), async {
            while counter.0.load(Ordering::SeqCst) != EVENT_COUNT {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("all queued events should be dispatched");
        assert_eq!(counter.0.load(Ordering::SeqCst), EVENT_COUNT);
    }
}
