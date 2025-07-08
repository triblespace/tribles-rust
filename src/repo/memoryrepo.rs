use std::collections::HashMap;
use std::convert::Infallible;

use crate::blob::{BlobSchema, MemoryBlobStore, ToBlob};
use crate::prelude::blobschemas::SimpleArchive;
use crate::prelude::*;
use crate::repo::{BranchStore, PushResult};
use crate::value::schemas::hash::Blake3;

use crate::value::schemas::hash::Handle;
use crate::value::ValueSchema;

#[derive(Debug)]
/// Simple in-memory implementation of [`BlobStore`] and [`BranchStore`].
///
/// Useful for unit tests or ephemeral repositories where persistence is not
/// required.
pub struct MemoryRepo {
    pub blobs: MemoryBlobStore<Blake3>,
    pub branches: HashMap<Id, Value<Handle<Blake3, SimpleArchive>>>,
}

impl Default for MemoryRepo {
    fn default() -> Self {
        Self {
            blobs: MemoryBlobStore::new(),
            branches: HashMap::new(),
        }
    }
}

impl crate::repo::BlobStorePut<Blake3> for MemoryRepo {
    type PutError = <MemoryBlobStore<Blake3> as crate::repo::BlobStorePut<Blake3>>::PutError;
    fn put<S, T>(&mut self, item: T) -> Result<Value<Handle<Blake3, S>>, Self::PutError>
    where
        S: BlobSchema + 'static,
        T: ToBlob<S>,
        Handle<Blake3, S>: ValueSchema,
    {
        self.blobs.put(item)
    }
}

impl crate::repo::BlobStore<Blake3> for MemoryRepo {
    type Reader = <MemoryBlobStore<Blake3> as crate::repo::BlobStore<Blake3>>::Reader;
    fn reader(&mut self) -> Self::Reader {
        self.blobs.reader()
    }
}

impl BranchStore<Blake3> for MemoryRepo {
    type BranchesError = Infallible;
    type HeadError = Infallible;
    type UpdateError = Infallible;

    type ListIter<'a> = std::vec::IntoIter<Result<Id, Self::BranchesError>>;

    fn branches<'a>(&'a self) -> Self::ListIter<'a> {
        self.branches
            .keys()
            .cloned()
            .map(Ok)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn head(
        &self,
        id: Id,
    ) -> Result<Option<Value<Handle<Blake3, SimpleArchive>>>, Self::HeadError> {
        Ok(self.branches.get(&id).cloned())
    }

    fn update(
        &mut self,
        id: Id,
        old: Option<Value<Handle<Blake3, SimpleArchive>>>,
        new: Value<Handle<Blake3, SimpleArchive>>,
    ) -> Result<PushResult<Blake3>, Self::UpdateError> {
        let current = self.branches.get(&id);
        if current != old.as_ref() {
            return Ok(PushResult::Conflict(current.cloned()));
        }
        self.branches.insert(id, new);
        Ok(PushResult::Success())
    }
}
