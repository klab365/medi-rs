use serde::{de::DeserializeOwned, Serialize};
use crate::error::Result;
use crate::error::Error;

pub type Encoded = Vec<u8>;
pub type Decoded = [u8];


pub fn serialize<T: Serialize + DeserializeOwned + Send>(value: &T) -> Result<Encoded> {
    bincode::serialize(value).map_err(|_| Error::SerializationError)
}

pub fn deserialize<T: DeserializeOwned>(value: &Decoded) -> Result<T> {
    bincode::deserialize(value).map_err(|_| Error::SerializationError)
}
