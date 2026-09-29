use medi_rs::medi_handler;

struct Handler;

impl Handler {
    #[medi_handler]
    async fn handle(&self, _: ()) {}
}

fn main() {}
