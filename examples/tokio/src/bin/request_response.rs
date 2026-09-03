use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
#[medi_command(return_type = String, error_type = medi_rs::Error)]
struct Greet {
    name: String,
}

#[medi_handler]
async fn greet(request: Greet) -> Result<String> {
    Ok(format!("Hello, {}!", request.name))
}

medi_module! { manifest greeting_manifest; commands { Greet => greet; } }
mediator! {
    pub struct GreetingMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [greeting_manifest];
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let greeting = GreetingMediator::new().send(Greet { name: "medi-rs".into() }).await?;
    println!("{greeting}");
    Ok(())
}
