mod from_resources;
mod resources;
mod resources_builder;

// - flatten
pub use from_resources::*;
pub use resources::*;
pub use resources_builder::*;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

type AnyMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;
