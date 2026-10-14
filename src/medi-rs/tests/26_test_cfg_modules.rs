use medi_rs::{MediCommand, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
struct Ping;

#[medi_handler]
async fn ping(_: Ping) -> Result<(), core::convert::Infallible> {
    Ok(())
}

medi_module! {
    manifest ping_module;
    commands { Ping => ping; }
}

mediator! {
    struct ConditionalMediator {
        event_queue_capacity: 4;
        event_workers: 1;
        modules: [
            #[cfg(any())] missing_first,
            #[cfg(all())] #[cfg(not(any()))] ping_module,
            #[cfg(any())] missing_last,
        ];
    }
}

mediator! {
    struct DisabledMediator {
        event_queue_capacity: 4;
        event_workers: 1;
        modules: [#[cfg(any())] missing];
    }
}

mediator! {
    struct EmptyMediator {
        event_queue_capacity: 4;
        event_workers: 1;
        modules: [];
    }
}

#[test]
fn only_enabled_modules_are_collected() {
    assert_eq!(ConditionalMediator::MODULE_COUNT, 1);
    assert_eq!(DisabledMediator::MODULE_COUNT, 0);
    assert_eq!(EmptyMediator::MODULE_COUNT, 0);
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn enabled_module_dispatches_commands() {
    ConditionalMediator::builder().build().send(Ping).await.unwrap();
}
