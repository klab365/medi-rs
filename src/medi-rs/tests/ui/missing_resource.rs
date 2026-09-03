use core::convert::Infallible;

use medi_rs::{MediCommand, mediator, medi_handler, medi_module};

#[derive(MediCommand)]
struct CreateUser;

#[derive(Clone)]
struct UserRepository;

#[medi_handler]
async fn create_user(_: UserRepository, _: CreateUser) -> Result<(), Infallible> {
    Ok(())
}

mod users {
    use super::medi_module;

    medi_module! {
        manifest users_manifest;
        commands { CreateUser => create_user; }
    }
}

use users::users_manifest;

mediator! {
    pub struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [users_manifest];
    }
}

fn main() {}
