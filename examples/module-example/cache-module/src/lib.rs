//! Cache feature: owns the refresh command handler.

use cache_contracts::{CacheRefreshed, RefreshCache};
use core::sync::atomic::{AtomicU32, Ordering};
use medi_rs::{Result, medi_handler, medi_module};

static GENERATION: AtomicU32 = AtomicU32::new(0);

#[medi_handler]
async fn refresh_cache(events: MediCtx<MediEvent<CacheRefreshed>>, _: RefreshCache) -> Result<u32> {
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    events.publish(CacheRefreshed { generation }).await?;
    Ok(generation)
}

#[medi_handler]
async fn report_refresh(event: CacheRefreshed) -> Result<()> {
    println!("cache refreshed; generation={}", event.generation);
    Ok(())
}

medi_module! {
    pub manifest cache_manifest;
    commands { cache_contracts::RefreshCache => crate::refresh_cache; }
    events { cache_contracts::CacheRefreshed => [crate::report_refresh]; }
}
