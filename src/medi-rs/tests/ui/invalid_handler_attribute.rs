use medi_rs::medi_handler;

#[medi_handler(unknown = [])]
async fn handler(_: ()) {}

fn main() {}
