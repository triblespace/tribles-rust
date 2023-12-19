pub mod handle;
pub mod semantic;
pub mod syntactic;

use std::{convert::TryInto, sync::Arc};

pub type Id = [u8; 16];
pub type Value = [u8; 32];
pub type Blob = Arc<[u8]>;

pub const ID_LEN: usize = 16;
pub const VALUE_LEN: usize = 32;

pub fn id_into_value(id: Id) -> Value {
    let mut data = [0; VALUE_LEN];
    data[16..=31].copy_from_slice(&id[..]);
    data
}

pub trait Idlike {
    fn from_id(id: Id) -> Self;
    fn into_id(&self) -> Id;
    fn factory() -> Self;
}

pub trait Valuelike {
    fn from_value(value: Value) -> Self;
    fn into_value(&self) -> Value;
}

pub trait Bloblike {
    fn from_blob(blob: Blob) -> Self;
    fn into_blob(&self) -> Blob;
}

impl<T: Idlike> Valuelike for T {
    fn from_value(value: Value) -> Self {
        Self::from_id(value[16..32].try_into().unwrap())
    }

    fn into_value(&self) -> Value {
        id_into_value(self.into_id())
    }
}
