use core::convert::Infallible;

use medi_rs::{MediCommand, medi_handler, medi_module, mediator};

mod users {
    use super::{Infallible, MediCommand, medi_handler, medi_module};

    #[derive(MediCommand)]
    pub struct CreateUser;

    #[medi_handler]
    async fn create_user(_: CreateUser) -> Result<(), Infallible> {
        Ok(())
    }

    medi_module! {
        manifest users_manifest;
        commands { CreateUser => crate::users::create_user; }
    }
}

use users::{CreateUser, users_manifest};

mediator! {
    struct AppMediator {
        event_queue_capacity: 1;
        event_workers: 1;
        modules: [users_manifest];
    }
}

#[test]
fn routes_to_a_private_handler_in_a_feature_module() {
    futures::executor::block_on(AppMediator::new().send(CreateUser)).unwrap();
}
