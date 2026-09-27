use medi_rs::{MediStreamRequest, medi_stream_handler};

#[derive(MediStreamRequest)]
#[medi_stream(item_type = u32)]
struct Numbers;

#[medi_stream_handler]
async fn numbers(_: Numbers) -> Result<(), core::convert::Infallible> {
    Ok(())
}

fn main() {}
