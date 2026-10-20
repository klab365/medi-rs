//! Public message contracts shared by feature modules.

use medi_rs::MediCommand;

#[derive(MediCommand)]
#[medi_command(return_type = u32, error_type = medi_rs::Error)]
pub struct RefreshCache;

#[derive(Clone)]
pub struct CacheRefreshed {
    pub generation: u32,
}
