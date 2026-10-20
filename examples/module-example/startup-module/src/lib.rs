//! Startup feature: depends on the cache contract, not the application mediator.

use cache_contracts::RefreshCache;
use medi_rs::{medi_module, medi_task};

#[medi_task]
async fn refresh_cache_at_startup(cache: MediCtx<MediCommand<RefreshCache>>) {
    match cache.send(RefreshCache).await {
        Ok(generation) => println!("startup refreshed cache; generation={generation}"),
        Err(error) => println!("startup cache refresh failed: {error:?}"),
    }
}

medi_module! {
    pub manifest startup_manifest;
    tasks { crate::refresh_cache_at_startup; }
}
