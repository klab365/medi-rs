use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub mod error;
pub mod from_resources;
pub mod resources;
pub mod resources_builder;

type AnyMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;
