use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Debug},
};

use bytes::Bytes;
use digest::{typenum::U32, Digest, OutputSizeUser};
use futures::{stream, Stream, StreamExt};

use crate::{types::Hash, BlobParseError, BlobSet};

#[derive(Debug)]
pub enum TransferError<ListErr, LoadErr, StoreErr> {
    List(ListErr),
    Load(LoadErr),
    Store(StoreErr),
}

impl<ListErr, LoadErr, StoreErr> fmt::Display for TransferError<ListErr, LoadErr, StoreErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to transfer blob")
    }
}

impl<ListErr, LoadErr, StoreErr> Error for TransferError<ListErr, LoadErr, StoreErr>
where
    ListErr: Debug + Error + 'static,
    LoadErr: Debug + Error + 'static,
    StoreErr: Debug + Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::List(e) => Some(e),
            Self::Load(e) => Some(e),
            Self::Store(e) => Some(e),
        }
    }
}

async fn transfer<'a, BS, BT, HS, HT, S>(
    source: &'a BS,
    target: &'a BT,
) -> impl Stream<
    Item = Result<(Hash<HS>, Hash<HT>), TransferError<BS::ListErr, BS::LoadErr, BT::StoreErr>>,
> + 'a
where
    BS: List<HS> + Pull<HS>,
    BT: Push<HT>,
    HS: 'static + Digest + OutputSizeUser<OutputSize = U32>,
    HT: 'static + Digest + OutputSizeUser<OutputSize = U32>,
{
    let l = source.list();
    let r = l.then(
        move |source_hash: Result<Hash<HS>, <BS as List<HS>>::ListErr>| async move {
            let source_hash = source_hash.map_err(|e| TransferError::List(e))?;
            let blob = source
                .pull(source_hash)
                .await
                .map_err(|e| TransferError::Load(e))?;
            let target_hash = target
                .push(blob)
                .await
                .map_err(|e| TransferError::Store(e))?;
            Ok((source_hash, target_hash))
        },
    );
    r
}

#[derive(Debug)]
enum GetError<E> {
    Load(E),
    Parse(BlobParseError),
}

pub trait List<H> {
    type ListErr;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Hash<H>, Self::ListErr>>;
}
pub trait Pull<H> {
    type LoadErr;

    async fn pull(&self, hash: Hash<H>) -> Result<Bytes, Self::LoadErr>;
}

pub trait Push<H> {
    type StoreErr;

    async fn push(&self, blob: Bytes) -> Result<Hash<H>, Self::StoreErr>;
}

pub trait Repo<H>: List<H> + Pull<H> + Push<H> {}

#[derive(Debug)]
pub struct NotFoundErr();

impl fmt::Display for NotFoundErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no blob for hash in blobset")
    }
}

impl Error for NotFoundErr {}

impl<H> List<H> for BlobSet<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    type ListErr = Infallible;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Hash<H>, Self::ListErr>> {
        stream::iter((&self).into_iter().map(|(hash, _)| Ok(hash)))
    }
}

impl<H> Pull<H> for BlobSet<H>
where
    H: Digest + OutputSizeUser<OutputSize = U32>,
{
    type LoadErr = NotFoundErr;

    async fn pull(&self, hash: Hash<H>) -> Result<Bytes, Self::LoadErr> {
        self.get_raw(hash)
            .map_or(Err(NotFoundErr()), |b| Ok(b.clone()))
    }
}
