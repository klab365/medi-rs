#![cfg(feature = "tokio")]

use medi_rs::{Result, medi_handler, medi_module, mediator};
use std::sync::{Arc, Mutex};

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

#[tokio::test]
async fn publish_should_process_published_event() {
    let queue = InMemoryMsgQueue::default();
    let mediator = Box::leak(Box::new(EventMediator::new((queue.clone(),))));
    mediator.start();
    mediator.publish(BaseEvent).await.unwrap();
    mediator.publish(BaseEvent).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(queue.0.lock().unwrap().len(), 6);
}
