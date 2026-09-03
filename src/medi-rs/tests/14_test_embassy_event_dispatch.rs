#![cfg(feature = "embassy")]

use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use futures::{pin_mut, poll};
use medi_rs::{medi_handler, medi_module, medi_task, mediator};

#[derive(Clone)]
struct EventObserved(u32);

#[derive(Clone, Copy)]
struct TaskState(&'static AtomicU32);

static OBSERVED_VALUE: AtomicU32 = AtomicU32::new(0);
static TASK_STARTED: AtomicU32 = AtomicU32::new(0);

#[medi_handler]
async fn observe_event(event: EventObserved) -> medi_rs::Result<()> {
    OBSERVED_VALUE.store(event.0, Ordering::Release);
    Ok(())
}

#[medi_task]
async fn initialize_board(_mediator: &EmbassyTestMediator, state: TaskState) {
    state.0.store(1, Ordering::Release);
}

medi_module! {
    manifest embassy_manifest;
    resources { TaskState; }
    events { EventObserved => [observe_event]; }
    tasks { initialize_board; }
}

mediator! {
    struct EmbassyTestMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [embassy_manifest];
    }
}

#[test]
fn embassy_queue_honors_the_generated_capacity() {
    let mediator = EmbassyTestMediator::new((TaskState(&TASK_STARTED),));

    futures::executor::block_on(mediator.publish(EventObserved(1))).expect("first event must fit");
    futures::executor::block_on(async {
        let publish = mediator.publish(EventObserved(2));
        pin_mut!(publish);
        assert!(poll!(publish).is_pending());
    });
}

#[test]
fn embassy_worker_dispatches_published_events() {
    let (started_tx, started_rx) = mpsc::sync_channel(1);

    thread::spawn(move || {
        let executor = Box::leak(Box::new(embassy_executor::Executor::new()));
        let mediator = Box::leak(Box::new(EmbassyTestMediator::new((TaskState(&TASK_STARTED),))));
        let mediator_for_test: &'static EmbassyTestMediator = mediator;

        executor.run(|spawner| {
            mediator.start(spawner);
            started_tx.send(mediator_for_test).expect("test must receive mediator");
        });
    });

    let mediator = started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Embassy executor must start");
    futures::executor::block_on(mediator.publish(EventObserved(42))).expect("event must be queued");

    let deadline = Instant::now() + Duration::from_secs(1);
    while OBSERVED_VALUE.load(Ordering::Acquire) != 42 || TASK_STARTED.load(Ordering::Acquire) != 1 {
        assert!(Instant::now() < deadline, "event worker or Embassy task did not run");
        thread::sleep(Duration::from_millis(1));
    }
}
