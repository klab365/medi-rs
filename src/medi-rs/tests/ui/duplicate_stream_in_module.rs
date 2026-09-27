use medi_rs::{mediator, medi_module};

medi_module! {
    manifest users_manifest;
    streams {
        SearchUsers => search_users;
        SearchUsers => search_users_again;
    }
}

mediator! {
    pub struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [users_manifest];
    }
}

fn main() {}
