//! Embassy-only startup task spawning.

/// A non-`Send` Embassy task spawner available only to `#[medi_startup]` hooks.
///
/// Unlike [`crate::SendSpawner`], this is not a mediator resource and cannot
/// outlive startup. It is supplied by `mediator.start(spawner)`, so it can
/// spawn tasks that are local to the Embassy executor.
#[cfg(feature = "embassy")]
#[derive(Clone, Copy)]
pub struct StartupSpawner {
    spawner: embassy_executor::Spawner,
}

#[cfg(feature = "embassy")]
impl StartupSpawner {
    /// Construct the temporary startup spawner used by generated code.
    #[doc(hidden)]
    pub const fn from(spawner: embassy_executor::Spawner) -> Self {
        Self { spawner }
    }

    /// Spawn an Embassy task on the executor that started the mediator.
    pub fn spawn<S>(&self, token: embassy_executor::SpawnToken<S>) {
        self.spawner.spawn(token);
    }
}

/// Placeholder type used by generated hooks on non-Embassy runtimes.
///
/// `#[medi_startup]` rejects this parameter unless the `embassy` feature is
/// enabled.
#[cfg(not(feature = "embassy"))]
pub struct StartupSpawner {
    _private: (),
}

#[cfg(not(feature = "embassy"))]
impl StartupSpawner {
    /// Construct the internal non-Embassy placeholder used by generated code.
    #[doc(hidden)]
    pub const fn unavailable() -> Self {
        Self { _private: () }
    }
}
