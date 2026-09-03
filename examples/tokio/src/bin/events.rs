use medi_rs::{Result, medi_handler, medi_module, mediator};
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, sleep};

#[derive(Clone)]
struct UserRegistered {
    email: String,
}
#[derive(Clone)]
struct EmailOutbox {
    sent: Arc<Mutex<Vec<String>>>,
}
#[medi_handler]
async fn send_welcome_email(outbox: EmailOutbox, event: UserRegistered) -> Result<()> {
    outbox
        .sent
        .lock()
        .unwrap()
        .push(format!("welcome email queued for {}", event.email));
    Ok(())
}
medi_module! { manifest events_manifest; resources { EmailOutbox; } events { UserRegistered => [send_welcome_email]; } }
mediator! { pub struct EventMediator { event_queue_capacity: 8; event_workers: 1; modules: [events_manifest]; } }
#[tokio::main]
async fn main() -> Result<()> {
    let outbox = EmailOutbox {
        sent: Arc::new(Mutex::new(Vec::new())),
    };
    let mediator = Box::leak(Box::new(EventMediator::new((outbox.clone(),))));
    mediator.start();
    mediator
        .publish(UserRegistered {
            email: "user@example.com".into(),
        })
        .await?;
    sleep(Duration::from_millis(50)).await;
    println!("{}", outbox.sent.lock().unwrap().join("\n"));
    Ok(())
}
