#![cfg(feature = "tokio")]

use std::sync::atomic::{AtomicU32, Ordering};

use medi_rs::stream::StreamExt;
use medi_rs::{
    MediCommand, MediStreamRequest, Result, StreamSender, medi_handler, medi_module, medi_stream_handler, medi_task,
    mediator,
};

static TASK_RESULT: AtomicU32 = AtomicU32::new(0);
static EVENT_VALUE: AtomicU32 = AtomicU32::new(0);

#[derive(MediCommand)]
#[medi_command(return_type = u32)]
struct Value;

#[derive(MediCommand)]
#[medi_command(return_type = u32, error_type = medi_rs::Error)]
struct UseContexts;

#[derive(Clone)]
struct ContextEvent(u32);

#[derive(MediStreamRequest)]
#[medi_stream(item_type = u32, error_type = medi_rs::Error)]
struct ContextStream;

#[medi_handler]
async fn value(_: Value) -> core::result::Result<u32, core::convert::Infallible> {
    Ok(40)
}

#[medi_handler]
async fn use_contexts(
    command: medi_rs::MediCtx<medi_rs::MediCommand<Value>>,
    event: medi_rs::MediCtx<medi_rs::MediEvent<ContextEvent>>,
    stream: medi_rs::MediCtx<medi_rs::MediStream<ContextStream>>,
    _: UseContexts,
) -> Result<u32> {
    event.publish(ContextEvent(2)).await?;
    let value = command.send(Value).await.map_err(|never| match never {})?;
    let output = stream.stream(ContextStream);
    let mut output = core::pin::pin!(output);
    Ok(value + output.next().await.unwrap().unwrap())
}

#[medi_handler]
async fn observe_event(event: ContextEvent) -> Result<()> {
    EVENT_VALUE.store(event.0, Ordering::Release);
    Ok(())
}

#[medi_stream_handler]
async fn context_stream(sender: StreamSender<'_, ContextStream>, _: ContextStream) -> Result<()> {
    sender.send(2).await;
    Ok(())
}

#[medi_task]
async fn context_task(
    command: medi_rs::MediCtx<medi_rs::MediCommand<Value>>,
    event: medi_rs::MediCtx<medi_rs::MediEvent<ContextEvent>>,
    stream: medi_rs::MediCtx<medi_rs::MediStream<ContextStream>>,
) {
    event.publish(ContextEvent(3)).await.unwrap();
    let value = command.send(Value).await.unwrap();
    let output = stream.stream(ContextStream);
    let mut output = core::pin::pin!(output);
    let item = output.next().await.unwrap().unwrap();
    TASK_RESULT.store(value + item, Ordering::Release);
}

medi_module! {
    manifest context_manifest;
    commands { Value => value; UseContexts => use_contexts; }
    events { ContextEvent => [observe_event]; }
    streams { ContextStream => context_stream; }
    tasks { context_task; }
}

mediator! {
    struct ContextMediator {
        event_queue_capacity: 2;
        event_workers: 1;
        modules: [context_manifest];
    }
}

#[tokio::test]
async fn handlers_and_tasks_can_use_all_typed_mediator_contexts() {
    TASK_RESULT.store(0, Ordering::Release);
    EVENT_VALUE.store(0, Ordering::Release);

    let mediator = Box::leak(Box::new(ContextMediator::builder().build()));
    mediator.start().expect("mediator must start");

    assert_eq!(mediator.send(UseContexts).await.unwrap(), 42);
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while TASK_RESULT.load(Ordering::Acquire) != 42 || EVENT_VALUE.load(Ordering::Acquire) != 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("context task and event handler must finish");
}
