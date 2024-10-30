use std::sync::{Arc, Mutex};

use medi_rs::{Bus, FromResources, HandlerResult, IntoEvent};

#[tokio::test]
async fn publish_should_process_published_event() {
    let message_queue = InMemoryMsgQueue::default();
    let bus = Bus::builder()
        .add_event_handler(base_event_handler1)
        .add_event_handler(base_event_handler2)
        .add_event_handler(base_event_handler3)
        .append_resources(message_queue.clone())
        .build()
        .unwrap();

    let event = BaseEvent;
    let watch = std::time::Instant::now();
    let res1 = bus.publish(event.clone()).await;
    let res2 = bus.publish(event).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await; // wait for event processing
    let duration = watch.elapsed();

    println!("Duration: {:?}", duration);
    message_queue.display_all();
    assert_eq!(message_queue.0.lock().unwrap().len(), 6);
    assert!(res1.is_ok());
    assert!(res2.is_ok());
}

#[derive(Clone)]
struct BaseEvent;
impl IntoEvent for BaseEvent {}

/// In-memory message queue for testing, if events are processed with the handlers
/// The order is not so important, but the handlers should be called
#[derive(Clone, Default)]
struct InMemoryMsgQueue(Arc<Mutex<Vec<String>>>);
impl FromResources for InMemoryMsgQueue {}
impl InMemoryMsgQueue {
    fn display_all(&self) {
        let queue = self.0.lock().unwrap();
        println!("Messages in the queue:");
        for item in queue.iter() {
            println!("{}", item);
        }
    }
}

async fn base_event_handler1(queue: InMemoryMsgQueue, _req: BaseEvent) -> HandlerResult<()> {
    queue.0.lock().unwrap().push("base_event_handler1".to_string());
    Ok(())
}

async fn base_event_handler2(queue: InMemoryMsgQueue, _req: BaseEvent) -> HandlerResult<()> {
    queue.0.lock().unwrap().push("base_event_handler2".to_string());
    Ok(())
}

async fn base_event_handler3(queue: InMemoryMsgQueue, _req: BaseEvent) -> HandlerResult<()> {
    queue.0.lock().unwrap().push("base_event_handler3".to_string());
    Ok(())
}
