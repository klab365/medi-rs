use core::convert::Infallible;

use medi_rs::{StaticCommand, medi_handler};
use medi_rs_macros::MediCommand;

#[derive(MediCommand)]
#[medi_command(return_type = u32)]
struct InfallibleCommand;

#[derive(MediCommand)]
#[medi_command(return_type = u32, error_type = DomainError)]
struct FallibleCommand;

struct DomainError;

#[derive(Clone)]
struct Repository;

#[medi_handler]
async fn handler_without_mediator(_: Repository, _: InfallibleCommand) -> Result<u32, Infallible> {
    Ok(1)
}

struct TestMediator;

#[medi_handler]
async fn handler_with_mediator(_: &TestMediator, _: InfallibleCommand) -> Result<u32, Infallible> {
    Ok(2)
}

fn assert_infallible<C: StaticCommand<Error = Infallible>>() {}
fn assert_domain_error<C: StaticCommand<Error = DomainError>>() {}

#[test]
fn derive_defaults_static_command_error_to_infallible() {
    assert_infallible::<InfallibleCommand>();
}

#[test]
fn derive_uses_the_declared_static_command_error() {
    assert_domain_error::<FallibleCommand>();
}

#[tokio::test]
async fn handler_attribute_generates_resource_invoker() {
    let value = __medi_handler_handler_without_mediator(&(), &(Repository, ()), InfallibleCommand)
        .await
        .unwrap();
    assert_eq!(value, 1);

    let value = __medi_handler_handler_with_mediator(&TestMediator, &(), InfallibleCommand)
        .await
        .unwrap();
    assert_eq!(value, 2);
}
