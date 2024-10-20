use medi_rs::{Bus, HandlerResult, IntoEvent};
use uuid::Uuid;

#[tokio::test]
async fn publish_should_process_published_event() {
    let bus = Bus::builder()
        .add_event_handler(base_event_handler1)
        .add_event_handler(base_event_handler2)
        .add_event_handler(base_event_handler3)
        .build()
        .unwrap();

    let event = BaseEvent::new();
    let watch = std::time::Instant::now();
    let res = bus.publish(event).await;
    let duration = watch.elapsed();

    println!("Duration: {:?}", duration);
    assert!(res.is_ok());
}

#[derive(Clone)]
struct BaseEvent(Uuid);

impl BaseEvent {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl IntoEvent for BaseEvent {
    fn get_id(&self) -> impl Into<String> {
        self.0.to_string()
    }
}

async fn base_event_handler1(_req: BaseEvent) -> HandlerResult<()> {
    println!("base_event_handler1");
    Ok(())
}

async fn base_event_handler2(_req: BaseEvent) -> HandlerResult<()> {
    println!("base_event_handler2");
    Ok(())
}

async fn base_event_handler3(_req: BaseEvent) -> HandlerResult<()> {
    println!("base_event_handler3");
    Ok(())
}
