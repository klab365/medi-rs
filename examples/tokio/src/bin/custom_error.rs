use medi_rs::{MediCommand, medi_handler, medi_module, mediator};

#[derive(MediCommand)]
#[medi_command(error_type = AppError)]
struct FailingCommand;
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("the command failed")]
    CommandFailed,
}
#[medi_handler]
async fn fail(_: FailingCommand) -> Result<(), AppError> {
    Err(AppError::CommandFailed)
}
medi_module! { manifest error_manifest; commands { FailingCommand => fail; } }
mediator! { pub struct ErrorMediator { event_queue_capacity: 1; event_workers: 1; modules: [error_manifest]; } }
#[tokio::main]
async fn main() {
    match ErrorMediator::new().send(FailingCommand).await {
        Ok(()) => println!("command succeeded"),
        Err(error) => println!("handler returned application error: {error}"),
    }
}
