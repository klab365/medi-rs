use medi_rs::{Result, medi_handler, medi_module, medi_shutdown, medi_startup, mediator};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct UserRegistered {
    email: String,
}
#[derive(Clone)]
struct EmailOutbox {
    sent: Arc<Mutex<Vec<String>>>,
}
#[medi_startup]
fn open_outbox(_mediator: &EventMediator, _outbox: &EmailOutbox) {
    println!("outbox is ready");
}

#[medi_shutdown]
fn close_outbox(_mediator: &EventMediator, _outbox: &EmailOutbox) {
    println!("outbox is closed");
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
medi_module! {
    manifest events_manifest;
    resources { outbox: EmailOutbox; }
    events { UserRegistered => [send_welcome_email]; }
    startup { open_outbox; }
    shutdown { close_outbox; }
}
mediator! { pub struct EventMediator { event_queue_capacity: 8; event_workers: 1; modules: [events_manifest]; } }
#[tokio::main]
async fn main() -> Result<()> {
    let outbox = EmailOutbox {
        sent: Arc::new(Mutex::new(Vec::new())),
    };
    let mediator = Box::leak(Box::new(EventMediator::builder().outbox(outbox.clone()).build()));
    mediator.start().expect("mediator must start");
    mediator
        .publish(UserRegistered {
            email: "user@example.com".into(),
        })
        .await?;
    // Shutdown drains the accepted event before the shutdown hook closes the outbox.
    mediator.shutdown().await?;
    println!("{}", outbox.sent.lock().unwrap().join("\n"));
    Ok(())
}
