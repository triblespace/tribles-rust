use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Debug},
};

use futures::{stream, Stream, StreamExt};

use crate::{
    blob::{schemas::UnknownBlob, Blob, BlobSchema},
    blobset::BlobSet,
    value::{
        schemas::hash::{Handle, HashProtocol},
        Value, ValueSchema,
    },
};

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
        (
            Value<Handle<HS, UnknownBlob>>,
            Value<Handle<HT, UnknownBlob>>,
        ),
        TransferError<<BS as List<HS>>::Err, <BS as Pull<HS>>::Err, <BT as Push<HT>>::Err>,
    >,
> + 'a
where
    BS: List<HS> + Pull<HS>,
    BT: Push<HT>,
    HS: 'static + HashProtocol,
    HT: 'static + HashProtocol,
{
    let l = source.list();
    let r = l.then(
        move |source_handle: Result<Value<Handle<HS, UnknownBlob>>, <BS as List<HS>>::Err>| async move {
            let source_handle = source_handle.map_err(|e| TransferError::List(e))?;
            let blob = source
                .pull(source_handle)
                .await
                .map_err(|e| TransferError::Load(e))?;
            let target_handle = target
                .push(blob)
                .await
                .map_err(|e| TransferError::Store(e))?;
            Ok((source_handle, target_handle))
        },
    );
    r
}

pub trait List<H: HashProtocol> {
    type Err;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>>;
}
pub trait Pull<H: HashProtocol> {
    type Err;

    fn pull<T>(
        &self,
        hash: Value<Handle<H, T>>,
    ) -> impl std::future::Future<Output = Result<Blob<T>, Self::Err>>
    where
        T: BlobSchema + 'static;
}

pub trait Push<H> {
    type Err;

    fn push<T>(
        &self,
        blob: Blob<T>,
    ) -> impl std::future::Future<Output = Result<Value<Handle<H, T>>, Self::Err>>
    where
        T: BlobSchema + 'static,
        Handle<H, T>: ValueSchema;
}

pub trait Repo<H: HashProtocol>: List<H> + Pull<H> + Push<H> {
    type ListErr;
    type PullErr;
    type PushErr;
}

impl<H, T> Repo<H> for T
where
    H: HashProtocol,
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
    H: HashProtocol,
{
    type Err = Infallible;

    fn list<'a>(&'a self) -> impl Stream<Item = Result<Value<Handle<H, UnknownBlob>>, Self::Err>> {
        stream::iter((&self).into_iter().map(|(&hash, _)| Ok(hash)))
    }
}

impl<H> Pull<H> for BlobSet<H>
where
    H: HashProtocol,
{
    type Err = NotFoundErr;

    async fn pull<T>(&self, handle: Value<Handle<H, T>>) -> Result<Blob<T>, Self::Err>
    where
        T: BlobSchema,
    {
        self.get(handle).ok_or(NotFoundErr())
    }
}
