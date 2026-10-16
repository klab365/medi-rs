use core::convert::Infallible;
use medi_rs::{MediCommand, MediContext, medi_module, mediator};

#[derive(MediCommand)]
#[medi_command(return_type = u32, error_type = Infallible)]
struct Increment;

struct Counter(u32);

impl Counter {
    async fn increment(
        &mut self,
        _context: &MediContext<'_, AppMediator>,
        _command: Increment,
    ) -> Result<u32, Infallible> {
        self.0 += 1;
        Ok(self.0)
    }
}

medi_module! {
    manifest counter_manifest;
    module Counter;
    mailbox_capacity: 8;
    commands { Increment => Counter::increment; }
}

mediator! {
    struct AppMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [counter_manifest];
    }
}

#[tokio::main]
async fn main() {
    let mediator = Box::leak(Box::new(AppMediator::builder().counter(Counter(0)).build()));
    mediator.start().expect("actor worker starts");
    println!("count: {}", mediator.send(Increment).await.expect("command succeeds"));
    mediator.shutdown().await.expect("actor worker shuts down");
}
