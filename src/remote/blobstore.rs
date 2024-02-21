use futures::Stream;

use crate::types::{handle::Handle, syntactic::Hash, Blob, BlobParseError, Bloblike};

#[derive(Debug)]
enum GetError<E> {
    Load(E),
    Parse(BlobParseError),
}

/*

#[derive(Debug)]
pub struct TransferError<H, E> {
    pub remaining: BlobSet<H>,
    pub causes: HashMap<Hash<H>, E>
}


impl<H, E> fmt::Display for TransferError<H, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to transfer {} blobs", self.remaining.len())
    }
}

impl<H, E> Error for TransferError<H, E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

async fn sync<BS, BT, HS, HT>(source: BS, target: BT) -> Result<(), TransferError<HS, BT::StoreErr>> 
where BS: BlobStore<HS>,
      BT: BlobStore<HT>{
        async fn put_all(&self, blobs: BlobSet<H>) -> Result<(), TransferError<H, Self::Err>> {
            let futures = FuturesUnordered::new();
    
            blobs.each_raw(|hash: Hash<H>, blob: Blob| {
                futures.push(async move {
                    if let Err(err) = self.store.put(&path, blob.clone()).await {
                        Some((hash, blob, err))
                    } else {
                        None
                    }
                });
            });
    
            let mut causes = std::collections::HashMap::new();
            let mut remaining = BlobSet::new();
    
            futures.for_each(|r| {
                if let Some((hash, blob, err)) = r {
                    causes.insert(hash, err);
                    remaining.put_raw(blob);
                }
                future::ready(())
            }).await;
    
            if causes.is_empty() {
                Ok(())
            } else {
                Err(PutError {
                    remaining,
                    causes
                })
            }
        }
}
*/

pub trait BlobStore<H> {
    type StoreErr;
    type LoadErr;
    type ListErr;
    type ListStream<'a>: Stream<Item = Result<Hash<H>, Self::ListErr>>
    where Self: 'a;

    async fn put_raw(&self, blob: Blob) -> Result<Hash<H>, Self::StoreErr>;
    async fn get_raw(&self, hash: Hash<H>) -> Result<Blob, Self::LoadErr>;
    async fn list<'a>(&'a self) -> Self::ListStream<'a>;

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
