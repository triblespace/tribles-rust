use std::{error::Error, fmt::{self, Debug}};

use futures::{stream::FuturesUnordered, Future, Stream, StreamExt};

use crate::types::{handle::Handle, syntactic::Hash, Blob, BlobParseError, Bloblike};

#[derive(Debug)]
pub enum TransferError<ListErr, LoadErr, StoreErr> {
    List(ListErr),
    Load(LoadErr),
    Store(StoreErr)
}

impl<ListErr, LoadErr, StoreErr> fmt::Display for TransferError<ListErr, LoadErr, StoreErr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to transfer blob")
    }
}

impl<ListErr, LoadErr, StoreErr> Error for TransferError<ListErr, LoadErr, StoreErr>
where ListErr: Debug + Error + 'static,
      LoadErr: Debug + Error + 'static,
      StoreErr: Debug + Error + 'static  {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::List(e) => Some(e),
            Self::Load(e)  => Some(e),
            Self::Store(e) => Some(e)
        }
    }
}

async fn transfer<'a, BS, BT, HS, HT, S>(source: &'a BS, target: &'a BT) -> impl Stream<Item = Result<(Hash<HS>, Hash<HT>), TransferError<BS::ListErr, BS::LoadErr, BT::StoreErr>>> + 'a
where BS: BlobStore<HS>,
      BT: BlobStore<HT>,
      HS: 'static,
      HT: 'static {    
    let l = source.list();
    let r = l.then(move |source_hash:Result<Hash<HS>, <BS as BlobStore<HS>>::ListErr>| {
        async move {
            let source_hash = source_hash.map_err(|e| TransferError::List(e))?;
            let blob = source.get_raw(source_hash).await
                .map_err(|e| TransferError::Load(e))?;
            let target_hash = target.put_raw(blob).await
                .map_err(|e| TransferError::Store(e))?;
            Ok((source_hash, target_hash))
        }
    });
    r
}

#[derive(Debug)]
enum GetError<E> {
    Load(E),
    Parse(BlobParseError),
}

pub trait BlobStore<H> {
    type StoreErr;
    type LoadErr;
    type ListErr;
    type ListStream<'a>: Stream<Item = Result<Hash<H>, Self::ListErr>>
    where Self: 'a;
    
    async fn put_raw(&self, blob: Blob) -> Result<Hash<H>, Self::StoreErr>;
    async fn get_raw(&self, hash: Hash<H>) -> Result<Blob, Self::LoadErr>;
    fn list<'a>(&'a self) -> Self::ListStream<'a>;
    
    async fn put<T>(&self, value: T) -> Result<Handle<H, T>, Self::StoreErr>
    where T: Bloblike {
        let blob: Blob = value.into_blob();
        let hash = self.put_raw(blob).await?;
        Ok(unsafe{ Handle::new(hash) })
    }
    async fn get<T>(&self, handle: Handle<H, T>) -> Result<T, GetError<Self::LoadErr>>
    where T: Bloblike {
        let blob = self.get_raw(handle.hash).await
        .map_err(|e| GetError::Load(e))?;
        T::from_blob(blob).map_err(|e| GetError::Parse(e))
    }
}
