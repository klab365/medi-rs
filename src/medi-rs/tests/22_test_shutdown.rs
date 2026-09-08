#![cfg(feature = "tokio")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use medi_rs::{Error, Result, medi_handler, medi_module, mediator};

#[derive(Clone)]
struct Event;

#[derive(Clone)]
struct Count(Arc<AtomicUsize>);

#[medi_handler]
async fn count(count: Count, _: Event) -> Result<()> {
    count.0.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

medi_module! {
    manifest shutdown_manifest;
    resources { Count; }
    events { Event => [count]; }
}

mediator! {
    struct ShutdownMediator {
        event_queue_capacity: 16;
        event_workers: 2;
        modules: [shutdown_manifest];
    }
}

#[tokio::test]
async fn shutdown_drains_events_and_rejects_new_publishes() {
    let count = Count(Arc::new(AtomicUsize::new(0)));
    let mediator = Box::leak(Box::new(ShutdownMediator::new(count.clone())));
    mediator.start();

    for _ in 0..8 {
        mediator.publish(Event).await.unwrap();
    }
    mediator.shutdown().await.unwrap();

    assert_eq!(count.0.load(Ordering::SeqCst), 8);
    assert!(matches!(
        mediator.publish(Event).await,
        Err(Error::EventPublishingError)
    ));
}

#[tokio::test]
async fn shutdown_without_events_completes() {
    let mediator = Box::leak(Box::new(ShutdownMediator::new(Count(Arc::new(AtomicUsize::new(0))))));
    mediator.start();
    mediator.shutdown().await.unwrap();
}
