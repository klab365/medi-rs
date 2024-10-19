mod from_resources;
mod resources;

// - flatten
pub use from_resources::*;
pub use resources::*;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

type AnyMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;
