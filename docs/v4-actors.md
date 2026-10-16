# medi-rs v4: Actor modules

This document records the v4 actor API and tracks the current implementation. The branch currently has generated module ownership, command `send`/`post`, actor event `publish`, and Tokio/WASM/Embassy reply backends. Legacy APIs remain temporarily while the migration is developed.

## Public API

A module is a user-defined state type with async methods. `medi_module!` declares the module type, its bounded mailbox capacity, and the routes handled by that module:

```rust
struct ButtonModule {
    input: ButtonInput,
    enabled: bool,
}

impl ButtonModule {
    async fn enable(
        &mut self,
        _context: &MediContext<'_, AppMediator>,
        _: EnableButton,
    ) -> core::result::Result<(), core::convert::Infallible> {
        self.enabled = true;
        Ok(())
    }
}

medi_module! {
    manifest button;
    module ButtonModule;
    mailbox_capacity: 8;
    commands { EnableButton => ButtonModule::enable; }
    events { ButtonPressed => [ButtonModule::on_button_pressed]; }
}
```

The application builder takes constructed module instances by value. Module fields remain private to the generated mediator. Applications use `mediator.send(command)`, `mediator.post(command)`, and `mediator.publish(event)`.

## Dispatch semantics

Each module has one actor worker that owns its module value and processes its mailbox sequentially. Handler methods receive `&mut self`; no mutex guard is held across `await` because ownership stays in the worker task.

- `send(command)` enqueues the command and awaits its typed response.
- `post(command)` attempts non-blocking enqueue and returns `PostError::MailboxFull(command)` if the bounded mailbox is full. It does not await handler execution.
- `publish(event)` attempts to enqueue a clone for each actor module declaring a matching event handler without waiting for mailbox capacity. Actor handlers run serially in their module workers; a full mailbox returns an event-publishing error.

A handler may post to its own module. Waiting on `send` to its own module is rejected at runtime with `DispatchError::SelfDispatch`; the actor cannot process that command until its current handler returns. Cross-module `send` cycles remain a possible deadlock. Event fan-out is sequential, so a shutdown or unavailable module during fan-out can result in partial delivery.

## Runtime constraints

Mailbox capacity is declared in the module manifest. Embassy uses it as a compile-time channel capacity and therefore rejects zero capacity at compile time. Tokio and WASM use bounded runtime channels; `try_build` reports zero capacity as `BuildError::InvalidMailboxCapacity`.

Tokio/WASM use typed oneshot replies. Embassy uses a fixed-capacity, allocation-free response-slot pool with generation keys and cancellation handling. Its concurrent outstanding `send` limit is the module mailbox capacity; exceeding that limit returns `DispatchError::ReplyCapacityExhausted`.

## Remaining migration

Actor command and event dispatch are implemented and covered under Tokio, WASM, and Embassy. The v4 migration is not complete: old resource injection/free-function routes still exist, and streams, startup/shutdown hooks, decorators, event failure reporting, full documentation/examples migration, and the workspace version bump remain to be addressed before calling this a v4 release.
