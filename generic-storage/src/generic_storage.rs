use bincode;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct SerializeError {
    pub message: String,
}

impl SerializeError {
    pub fn new(msg: &str) -> SerializeError {
        SerializeError { message: msg.to_string() }
    }
}

pub trait Serializer {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where T: BorshSerialize + Serialize;

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where T: BorshDeserialize + for<'de> Deserialize<'de>;
}

pub struct Borsh;

impl Serializer for Borsh {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where
        T: BorshSerialize + Serialize,
    {
        borsh::to_vec(value).map_err(|e| SerializeError::new(&e.to_string()))
    }

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where
        T: BorshDeserialize + for<'de> Deserialize<'de>,
    {
        T::try_from_slice(bytes).map_err(|e| SerializeError::new(&e.to_string()))
    }
}

pub struct Bincode;

impl Serializer for Bincode {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where
        T: BorshSerialize + Serialize,
    {
        bincode::serialize(value).map_err(|e| SerializeError::new(&e.to_string()))
    }

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where 
        T: BorshDeserialize + for<'de> Deserialize<'de>,
    {
        bincode::deserialize(bytes).map_err(|e| SerializeError::new(&e.to_string()))
    }
}

pub struct Json;

impl Serializer for Json {
    fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>, SerializeError>
    where 
        T: Serialize,
    {
        serde_json::to_vec(value).map_err(|e| SerializeError::new(&e.to_string()))
    }

    fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T, SerializeError>
    where 
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_slice(bytes).map_err(|e| SerializeError::new(&e.to_string()))
    }
}

pub struct Storage<T, S>
where 
    T: BorshSerialize + Serialize + BorshDeserialize + for<'de> Deserialize<'de>,
    S: Serializer 
{
    format: S,
    data: Vec<u8>,
    _marker: PhantomData<T>
}

impl<T, S> Storage<T, S>
where 
    T: BorshSerialize + Serialize + BorshDeserialize + for<'de> Deserialize<'de>,
    S: Serializer
{
    pub fn new(serializer: S) -> Storage<T, S> {
        Storage { 
            format: serializer, 
            data: Vec::new(), 
            _marker: PhantomData 
        }   
    }

    pub fn save(&mut self, value: &T) -> Result<(), SerializeError> {
        match self.format.to_bytes(value) {
            Ok(data) => {
                self.data = data;
                self._marker = PhantomData::<T>;
            },
            Err(e) => return Err(e),
        }
        
        Ok(())
    }

    pub fn load(&self) -> Result<T, SerializeError> {
        match self.format.from_bytes(&self.data) {
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }

    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    pub fn convert_to_other_format<S2: Serializer>(&self, other_serializer: S2) -> Storage<T, S2> {
        Storage { 
            format: other_serializer, 
            data: self.data.clone(), 
            _marker: PhantomData 
        }   
    }
}
