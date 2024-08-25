use crate::{valueschemas::Hash, Value};

#[derive(Debug)]
pub enum CommitResult<H> {
    Success(),
    Conflict(Option<Value<Hash<H>>>),
}

pub trait Head<H> {
    type CheckoutErr;
    type CommitErr;

    async fn checkout(&self) -> Result<Option<Value<Hash<H>>>, Self::CheckoutErr>;
    async fn commit(
        &self,
        old: Option<Value<Hash<H>>>,
        new: Value<Hash<H>>,
    ) -> Result<CommitResult<H>, Self::CommitErr>;
}
