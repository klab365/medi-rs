use medi_rs::MediStreamRequest;

#[derive(MediStreamRequest)]
#[medi_stream(error_type = u8)]
struct Numbers;

fn main() {}
