use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use std::marker::PhantomData;
use wincode::config::DefaultConfig;

#[derive(Debug)]
pub struct SerializeError {
    pub message: String,
}

impl SerializeError {
    pub fn new(msg: &str) -> SerializeError {
        SerializeError {
            message: msg.to_string(),
        }
    }
}

pub trait WincodeRead: for<'de> wincode::SchemaRead<'de, DefaultConfig, Dst = Self> {}
impl<T> WincodeRead for T where T: for<'de> wincode::SchemaRead<'de, DefaultConfig, Dst = Self> {}


pub trait WincodeWrite: wincode::SchemaWrite<DefaultConfig, Src = Self> {}
impl<T> WincodeWrite for T where T: wincode::SchemaWrite<DefaultConfig, Src = Self> {}

pub trait Serializer {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where
        T: BorshSerialize + SerdeSerialize + WincodeWrite;

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where
        T: BorshDeserialize + for<'de> SerdeDeserialize<'de> + WincodeRead;
}

pub struct Borsh;

impl Serializer for Borsh {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where
        T: BorshSerialize + SerdeSerialize + WincodeWrite,
    {
        borsh::to_vec(value).map_err(|e| SerializeError::new(&e.to_string()))
    }

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where
        T: BorshDeserialize + for<'de> SerdeDeserialize<'de> + WincodeRead,
    {
        T::try_from_slice(bytes).map_err(|e| SerializeError::new(&e.to_string()))
    }
}

pub struct WincodeSerializer;

impl Serializer for WincodeSerializer {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where
        T: BorshSerialize + SerdeSerialize + WincodeWrite,
    {
        wincode::serialize(value).map_err(|e| SerializeError::new(&e.to_string()))
    }

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where
        T: BorshDeserialize + for<'de> SerdeDeserialize<'de> + WincodeRead,
    {
        wincode::deserialize(bytes).map_err(|e| SerializeError::new(&e.to_string()))
    }
}

pub struct Json;

impl Serializer for Json {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where
        T: BorshSerialize + SerdeSerialize + WincodeWrite,
    {
        serde_json::to_vec(value).map_err(|e| SerializeError::new(&e.to_string()))
    }

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where
        T: BorshDeserialize + for<'de> SerdeDeserialize<'de> + WincodeRead,
    {
        serde_json::from_slice(bytes).map_err(|e| SerializeError::new(&e.to_string()))
    }
}

pub struct Storage<T, S>
where
    T: BorshSerialize
        + SerdeSerialize
        + BorshDeserialize
        + for<'de> SerdeDeserialize<'de>
        + WincodeWrite
        + WincodeRead,
    S: Serializer,
{
    format: S,
    data: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T, S> Storage<T, S>
where
    T: BorshSerialize
        + SerdeSerialize
        + BorshDeserialize
        + for<'de> SerdeDeserialize<'de>
        + WincodeWrite
        + WincodeRead,
    S: Serializer,
{
    pub fn new(serializer: S) -> Storage<T, S> {
        Storage {
            format: serializer,
            data: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn save(&mut self, value: &T) -> Result<(), SerializeError> {
        self.data = self.format.to_bytes(value)?;
        Ok(())
    }

    pub fn load(&self) -> Result<T, SerializeError> {
        self.format.from_bytes(&self.data)
    }

    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    pub fn convert_to_other_format<S2: Serializer>(&self, other_serializer: S2) -> Storage<T, S2> {
        Storage {
            format: other_serializer,
            data: self.data.clone(),
            _marker: PhantomData,
        }
    }
}