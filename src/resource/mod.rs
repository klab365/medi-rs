use std::{any::{Any, TypeId}, collections::HashMap};

pub mod resources_builder;
pub mod resources;
pub mod from_resources;
pub mod error;


type AnyMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;
