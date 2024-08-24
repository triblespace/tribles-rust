use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Debug},
};

use anybytes::Bytes;
use digest::{typenum::U32, Digest};
use futures::{stream, Stream, StreamExt};

use crate::{schemas::Hash, BlobParseError, BlobSet, Value};

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

pub async fn transfer<'a, BS, BT, HS, HT, S>(
    source: &'a BS,
    target: &'a BT,
) -> impl Stream<
    Item = Result<
        (Value<Hash<HS>>, Value<Hash<HT>>),
        TransferError<<BS as List<HS>>::Err, <BS as Pull<HS>>::Err, <BT as Push<HT>>::Err>,
    >,
> + 'a
where
    BS: List<HS> + Pull<HS>,
    BT: Push<HT>,
    HS: 'static + Digest<OutputSize = U32>,
    HT: 'static + Digest<OutputSize = U32>,
{
    let l = source.list();
    let r = l.then(
        move |source_hash: Result<Value<Hash<HS>>, <BS as List<HS>>::Err>| async move {
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
pub enum GetError<E> {
    Load(E),
    Parse(BlobParseError),
}

pub trait List<H> {
    type Err;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Hash<H>>, Self::Err>>;
}
pub trait Pull<H> {
    type Err;

    async fn pull(&self, hash: Value<Hash<H>>) -> Result<Bytes, Self::Err>;
}

pub trait Push<H> {
    type Err;

    async fn push(&self, blob: Bytes) -> Result<Value<Hash<H>>, Self::Err>;
}

pub trait Repo<H>: List<H> + Pull<H> + Push<H> {
    type ListErr;
    type PullErr;
    type PushErr;
}

impl<H, T> Repo<H> for T
where
    H: Digest<OutputSize = U32>,
    T: List<H> + Pull<H> + Push<H>,
{
    type ListErr = <Self as List<H>>::Err;
    type PullErr = <Self as Pull<H>>::Err;
    type PushErr = <Self as Push<H>>::Err;
}

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
    H: Digest<OutputSize = U32>,
{
    type Err = Infallible;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Hash<H>>, Self::Err>> {
        stream::iter((&self).into_iter().map(|(&hash, _)| Ok(hash)))
    }
}

impl<H> Pull<H> for BlobSet<H>
where
    H: Digest<OutputSize = U32>,
{
    type Err = NotFoundErr;

    async fn pull(&self, hash: Value<Hash<H>>) -> Result<Bytes, Self::Err> {
        self.get_raw(hash)
            .map_or(Err(NotFoundErr()), |b| Ok(b.clone()))
    }
}
