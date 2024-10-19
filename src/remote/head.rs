use crate::value::{
    schemas::hash::{Hash, HashProtocol},
    Value,
};

#[derive(Debug)]
pub enum CommitResult<H>
where
    H: HashProtocol,
{
    Success(),
    Conflict(Option<Value<Hash<H>>>),
}

pub trait Head<H: HashProtocol> {
    type CheckoutErr;
    type CommitErr;

    fn checkout(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<Value<Hash<H>>>, Self::CheckoutErr>>;
    fn commit(
        &self,
        old: Option<Value<Hash<H>>>,
        new: Value<Hash<H>>,
    ) -> impl std::future::Future<Output = Result<CommitResult<H>, Self::CommitErr>>;
}
