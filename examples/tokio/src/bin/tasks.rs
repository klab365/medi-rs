use medi_rs::{medi_module, medi_task, mediator};
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use tokio::time::{Duration, sleep};

#[derive(Clone)]
struct TickCount(Arc<AtomicU32>);

#[medi_task]
async fn count_ticks(_mediator: &TaskMediator, ticks: TickCount) {
    for _ in 0..3 {
        sleep(Duration::from_millis(10)).await;
        ticks.0.fetch_add(1, Ordering::Relaxed);
    }
}

medi_module! {
    manifest task_manifest;
    resources { TickCount; }
    tasks { count_ticks; }
}

mediator! {
    pub struct TaskMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [task_manifest];
    }
}

#[tokio::main]
async fn main() {
    let ticks = TickCount(Arc::new(AtomicU32::new(0)));
    let mediator = Box::leak(Box::new(TaskMediator::new((ticks.clone(),))));
    mediator.start();

    sleep(Duration::from_millis(50)).await;
    println!("task completed {} ticks", ticks.0.load(Ordering::Relaxed));
}
