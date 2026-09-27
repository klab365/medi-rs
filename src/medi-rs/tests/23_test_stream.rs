#![cfg(any(feature = "tokio", feature = "wasm", feature = "embassy"))]

use core::pin::pin;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use futures::{executor::block_on, poll};
use medi_rs::stream::{StreamExt, TryStreamExt};
use medi_rs::{
    MediCommand, MediStreamRequest, StreamRequest, StreamSender, medi_handler, medi_module, medi_stream_handler,
    mediator,
};

#[derive(Debug, PartialEq, Eq)]
enum SearchError {
    Unavailable,
}

#[derive(Clone)]
struct Prefix(&'static str);

struct UserDirectory {
    users: Vec<&'static str>,
}

#[derive(MediStreamRequest)]
#[medi_stream(item_type = String, error_type = SearchError, capacity = 2)]
struct SearchUsers {
    query: &'static str,
}

#[medi_stream_handler]
async fn search_users(
    _mediator: &StreamMediator,
    directory: &UserDirectory,
    prefix: Prefix,
    sender: StreamSender<'_, SearchUsers>,
    request: SearchUsers,
) -> Result<(), SearchError> {
    for user in directory.users.iter().filter(|user| user.contains(request.query)) {
        sender.send(format!("{}{user}", prefix.0)).await;
    }
    Ok(())
}

#[derive(MediStreamRequest)]
#[medi_stream(item_type = u32, error_type = SearchError)]
struct FailAfter {
    items: u32,
}

#[medi_stream_handler]
async fn fail_after(sender: StreamSender<'_, FailAfter>, request: FailAfter) -> Result<(), SearchError> {
    for item in 0..request.items {
        sender.send(item).await;
    }
    Err(SearchError::Unavailable)
}

/// Observes progress and cancellation of the `count` handler.
#[derive(Clone, Default)]
struct Probe {
    started: Arc<AtomicBool>,
    sent: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
}

struct SetOnDrop(Arc<AtomicBool>);

impl Drop for SetOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[derive(MediStreamRequest)]
#[medi_stream(item_type = usize, capacity = 1)]
struct Count {
    to: usize,
}

#[medi_stream_handler]
async fn count<'a>(
    probe: Probe,
    sender: StreamSender<'a, Count>,
    request: Count,
) -> Result<(), core::convert::Infallible> {
    probe.started.store(true, Ordering::Release);
    let _guard = SetOnDrop(probe.dropped.clone());
    for item in 0..request.to {
        sender.send(item).await;
        probe.sent.fetch_add(1, Ordering::AcqRel);
    }
    Ok(())
}

#[derive(MediStreamRequest)]
#[medi_stream(item_type = T, capacity = 4)]
struct Repeat<T: Clone + Send + 'static> {
    value: T,
    times: usize,
}

#[medi_stream_handler]
async fn repeat_numbers(
    sender: StreamSender<'_, Repeat<u8>>,
    request: Repeat<u8>,
) -> Result<(), core::convert::Infallible> {
    for _ in 0..request.times {
        sender.send(request.value).await;
    }
    Ok(())
}

#[derive(MediCommand)]
#[medi_command(return_type = usize)]
struct CountUsers;

#[medi_handler]
async fn count_users(
    _mediator: &StreamMediator,
    directory: &UserDirectory,
    _: CountUsers,
) -> Result<usize, core::convert::Infallible> {
    Ok(directory.users.len())
}

mod numbers {
    use super::*;

    #[medi_stream_handler]
    async fn private_numbers(
        sender: StreamSender<'_, PrivateNumbers>,
        request: PrivateNumbers,
    ) -> Result<(), core::convert::Infallible> {
        for item in 0..request.0 {
            sender.send(item).await;
        }
        Ok(())
    }

    #[derive(MediStreamRequest)]
    #[medi_stream(item_type = u16)]
    pub struct PrivateNumbers(pub u16);

    medi_module! {
        manifest numbers_manifest;
        streams { crate::numbers::PrivateNumbers => crate::numbers::private_numbers; }
    }
}

use numbers::numbers_manifest;

medi_module! {
    manifest users_manifest;
    resources { UserDirectory; Prefix; Probe; }
    commands { CountUsers => count_users; }
    streams {
        SearchUsers => search_users;
        FailAfter => fail_after;
        Count => count;
        Repeat<u8> => repeat_numbers;
    }
}

mediator! {
    struct StreamMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [users_manifest, numbers_manifest];
    }
}

fn mediator(probe: Probe) -> StreamMediator {
    StreamMediator::new(
        UserDirectory {
            users: vec!["Ada Lovelace", "Alan Turing", "Ada Yonath", "Grace Hopper"],
        },
        Prefix("user:"),
        probe,
    )
}

#[test]
fn stream_request_derive_implements_metadata() {
    assert_eq!(<SearchUsers as StreamRequest>::CAPACITY, 2);
    assert_eq!(<FailAfter as StreamRequest>::CAPACITY, 1);
    assert_eq!(<Repeat<String> as StreamRequest>::CAPACITY, 4);
    let _: Option<<Repeat<String> as StreamRequest>::Item> = Some(String::new());
    let _: Option<<Count as StreamRequest>::Error> = None::<core::convert::Infallible>;
}

#[test]
fn stream_yields_items_with_injected_resources_in_order() {
    let mediator = mediator(Probe::default());

    let users: Vec<_> = block_on(mediator.stream(SearchUsers { query: "Ada" }).collect());

    assert_eq!(
        users,
        vec![Ok("user:Ada Lovelace".to_owned()), Ok("user:Ada Yonath".to_owned())]
    );
}

#[test]
fn stream_supports_while_let_consumption() {
    let mediator = mediator(Probe::default());

    let found = block_on(async {
        let mut users = pin!(mediator.stream(SearchUsers { query: "a" }));
        let mut found = Vec::new();
        while let Some(user) = users.next().await {
            found.push(user?);
        }
        Ok::<_, SearchError>(found)
    });

    assert_eq!(found.unwrap().len(), 4);
}

#[test]
fn try_for_each_consumes_without_pinning() {
    let mediator = mediator(Probe::default());

    let mut found = Vec::new();
    let result = block_on(mediator.stream(SearchUsers { query: "Ada" }).try_for_each(|user| {
        found.push(user);
        async { Ok(()) }
    }));

    assert_eq!(result, Ok(()));
    assert_eq!(found, vec!["user:Ada Lovelace", "user:Ada Yonath"]);
}

#[test]
fn try_for_each_returns_handler_error_after_processing_items() {
    let mediator = mediator(Probe::default());

    let mut processed = Vec::new();
    let result = block_on(mediator.stream(FailAfter { items: 2 }).try_for_each(|item| {
        processed.push(item);
        async { Ok(()) }
    }));

    assert_eq!(result, Err(SearchError::Unavailable));
    assert_eq!(processed, vec![0, 1]);
}

#[test]
fn try_collect_returns_items_or_the_handler_error() {
    let mediator = mediator(Probe::default());

    let users: Result<Vec<_>, _> = block_on(mediator.stream(SearchUsers { query: "Grace" }).try_collect());
    let failed: Result<Vec<_>, _> = block_on(mediator.stream(FailAfter { items: 2 }).try_collect());

    assert_eq!(users, Ok(vec!["user:Grace Hopper".to_owned()]));
    assert_eq!(failed, Err(SearchError::Unavailable));
}

#[test]
fn empty_stream_ends_without_items() {
    let mediator = mediator(Probe::default());

    let users: Vec<_> = block_on(mediator.stream(SearchUsers { query: "nobody" }).collect());

    assert!(users.is_empty());
}

#[test]
fn handler_error_is_yielded_once_after_all_sent_items() {
    let mediator = mediator(Probe::default());

    let items: Vec<_> = block_on(mediator.stream(FailAfter { items: 3 }).collect());

    assert_eq!(items, vec![Ok(0), Ok(1), Ok(2), Err(SearchError::Unavailable)]);
}

#[test]
fn handler_error_without_items_is_the_only_item() {
    let mediator = mediator(Probe::default());

    let items: Vec<_> = block_on(mediator.stream(FailAfter { items: 0 }).collect());

    assert_eq!(items, vec![Err(SearchError::Unavailable)]);
}

#[test]
fn stream_is_lazy_until_polled() {
    let probe = Probe::default();
    let mediator = mediator(probe.clone());

    let stream = mediator.stream(Count { to: 3 });
    assert!(!probe.started.load(Ordering::Acquire));
    drop(stream);

    assert!(!probe.started.load(Ordering::Acquire));
}

#[test]
fn handler_waits_for_consumer_when_channel_is_full() {
    let probe = Probe::default();
    let mediator = mediator(probe.clone());

    block_on(async {
        let mut numbers = pin!(mediator.stream(Count { to: 100 }));
        assert_eq!(numbers.next().await, Some(Ok(0)));
        // Capacity 1: at most the consumed item and one buffered item were sent.
        assert!(probe.sent.load(Ordering::Acquire) <= 2);
    });
}

#[test]
fn dropping_the_stream_cancels_the_handler_and_discards_buffered_items() {
    let probe = Probe::default();
    let mediator = mediator(probe.clone());

    block_on(async {
        let mut numbers = pin!(mediator.stream(Count { to: 100 }));
        assert_eq!(numbers.next().await, Some(Ok(0)));
        assert!(!probe.dropped.load(Ordering::Acquire));
    });
    assert!(probe.dropped.load(Ordering::Acquire));
    let sent_before_cancel = probe.sent.load(Ordering::Acquire);

    let numbers: Vec<_> = block_on(mediator.stream(Count { to: 2 }).collect());

    assert_eq!(numbers, vec![Ok(0), Ok(1)]);
    assert_eq!(probe.sent.load(Ordering::Acquire), sent_before_cancel + 2);
}

#[test]
fn second_stream_of_a_route_waits_for_the_active_stream() {
    let mediator = mediator(Probe::default());

    block_on(async {
        // Boxed so that `drop(first)` drops the stream itself, not a pin reference.
        let mut first = Box::pin(mediator.stream(Count { to: 10 }));
        assert_eq!(first.next().await, Some(Ok(0)));

        let mut second = Box::pin(mediator.stream(Count { to: 1 }));
        assert!(poll!(second.next()).is_pending());

        // Other routes are independent of the active `Count` stream.
        let repeated: Vec<_> = mediator.stream(Repeat { value: 7u8, times: 2 }).collect().await;
        assert_eq!(repeated, vec![Ok(7), Ok(7)]);

        drop(first);
        assert_eq!(second.next().await, Some(Ok(0)));
        assert_eq!(second.next().await, None);
    });
}

#[test]
fn streams_commands_and_private_module_routes_share_one_mediator() {
    let mediator = mediator(Probe::default());

    assert_eq!(block_on(mediator.send(CountUsers)), Ok(4));
    let numbers: Vec<_> = block_on(mediator.stream(numbers::PrivateNumbers(3)).collect());

    assert_eq!(numbers, vec![Ok(0), Ok(1), Ok(2)]);
}

#[test]
fn stream_after_completion_can_be_reopened() {
    let mediator = mediator(Probe::default());

    for _ in 0..3 {
        let items: Vec<_> = block_on(mediator.stream(FailAfter { items: 1 }).collect());
        assert_eq!(items, vec![Ok(0), Err(SearchError::Unavailable)]);
    }
}

#[cfg(feature = "tokio")]
mod tokio_runtime {
    use super::*;
    use std::time::Duration;

    #[derive(MediStreamRequest)]
    #[medi_stream(item_type = u64, capacity = 2)]
    struct Ticks(u64);

    #[medi_stream_handler]
    async fn ticks(sender: StreamSender<'_, Ticks>, request: Ticks) -> Result<(), core::convert::Infallible> {
        for tick in 0..request.0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            sender.send(tick).await;
        }
        Ok(())
    }

    medi_module! { manifest ticks_manifest; streams { Ticks => crate::tokio_runtime::ticks; } }
    mediator! { struct TickMediator { event_queue_capacity: 1; event_workers: 1; modules: [ticks_manifest]; } }

    #[tokio::test]
    async fn tokio_stream_is_send_and_can_be_consumed_in_a_spawned_task() {
        let mediator: &'static TickMediator = Box::leak(Box::new(TickMediator::new()));

        let stream = mediator.stream(Ticks(3));
        let ticks = tokio::spawn(async move { stream.collect::<Vec<_>>().await })
            .await
            .unwrap();

        assert_eq!(ticks, vec![Ok(0), Ok(1), Ok(2)]);
    }
}

#[cfg(feature = "embassy")]
mod embassy_runtime {
    use super::*;
    use std::{sync::mpsc, thread, time::Duration};

    #[derive(MediStreamRequest)]
    #[medi_stream(item_type = u32, capacity = 2)]
    struct Samples(u32);

    #[medi_stream_handler]
    async fn samples(sender: StreamSender<'_, Samples>, request: Samples) -> Result<(), core::convert::Infallible> {
        for sample in 0..request.0 {
            sender.send(sample * 10).await;
        }
        Ok(())
    }

    medi_module! { manifest samples_manifest; streams { crate::embassy_runtime::Samples => crate::embassy_runtime::samples; } }
    mediator! { struct SampleMediator { event_queue_capacity: 1; event_workers: 1; modules: [samples_manifest]; } }

    #[embassy_executor::task]
    async fn consume(mediator: &'static SampleMediator, results: mpsc::SyncSender<Vec<u32>>) {
        let mut samples = pin!(mediator.stream(Samples(5)));
        let mut received = Vec::new();
        while let Some(sample) = samples.next().await {
            received.push(sample.unwrap());
        }
        results.send(received).unwrap();
    }

    #[test]
    fn embassy_task_consumes_a_stream_from_static_storage() {
        let (results_tx, results_rx) = mpsc::sync_channel(1);

        thread::spawn(move || {
            let executor = Box::leak(Box::new(embassy_executor::Executor::new()));
            let mediator: &'static SampleMediator = Box::leak(Box::new(SampleMediator::new()));
            executor.run(|spawner| spawner.spawn(consume(mediator, results_tx).unwrap()));
        });

        let received = results_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("Embassy task must consume the stream");
        assert_eq!(received, vec![0, 10, 20, 30, 40]);
    }
}
