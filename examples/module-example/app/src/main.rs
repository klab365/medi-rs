use cache_module::cache_manifest;
use medi_rs::mediator;
use startup_module::startup_manifest;

mediator! {
    struct ApplicationMediator {
        event_queue_capacity: 8;
        event_workers: 1;
        modules: [cache_manifest, startup_manifest];
    }
}

#[tokio::main]
async fn main() {
    let mediator = Box::leak(Box::new(ApplicationMediator::builder().build()));
    mediator.start().expect("mediator should start");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    mediator.shutdown().await.expect("mediator should shut down");
}
