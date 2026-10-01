use core::convert::Infallible;

use medi_rs::{MediCommand, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
struct Ping;

#[medi_handler]
async fn ping(_: Ping) -> Result<(), Infallible> {
    Ok(())
}

medi_module! {
    pub manifest public_manifest;
    commands { crate::Ping => crate::ping; }
}

mediator! {
    struct AppMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [public_manifest];
    }
}

#[test]
fn public_manifest_composes_a_private_handler() {
    futures::executor::block_on(AppMediator::builder().build().send(Ping)).unwrap();
}
