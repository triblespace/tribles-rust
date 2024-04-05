use crate::types::Hash;

#[derive(Debug)]
pub enum CommitResult<H> {
    Success(),
    Conflict(Option<Hash<H>>),
}

pub trait Head<H> {
    type CheckoutErr;
    type CommitErr;

    async fn checkout(&self) -> Result<Option<Hash<H>>, Self::CheckoutErr>;
    async fn commit(
        &self,
        old: Option<Hash<H>>,
        new: Hash<H>,
    ) -> Result<CommitResult<H>, Self::CommitErr>;
}
