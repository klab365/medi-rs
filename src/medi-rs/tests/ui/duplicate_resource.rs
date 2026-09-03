use medi_rs::{mediator, medi_module};

mod first {
    use super::medi_module;

    medi_module! {
        manifest first_manifest;
        resources { Database; }
    }
}

mod second {
    use super::medi_module;

    medi_module! {
        manifest second_manifest;
        resources { Database; }
    }
}

use first::first_manifest;
use second::second_manifest;

mediator! {
    pub struct AppMediator {
        event_queue_capacity: 16;
        event_workers: 1;
        modules: [first_manifest, second_manifest];
    }
}

fn main() {}
