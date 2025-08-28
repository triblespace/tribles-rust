use anybytes::Bytes;
use tempfile::tempdir;
use tribles::blob::{schemas::UnknownBlob, Blob};
use tribles::prelude::{BlobStore, BlobStoreGet, BlobStorePut};
use tribles::repo::pile::Pile;
use tribles::value::schemas::hash::Blake3;

#[test]
fn put_and_get_empty_blob() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("pile.pile");

    let handle = {
        let mut pile: Pile<Blake3> = Pile::open(&path).unwrap();
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(Vec::<u8>::new()));
        let handle = pile.put(blob).unwrap();
        pile.flush().unwrap();
        pile.close().unwrap();
        handle
    };

    let mut reopened: Pile<Blake3> = Pile::open(&path).unwrap();
    let blob = reopened
        .reader()
        .unwrap()
        .get::<Blob<UnknownBlob>, _>(handle)
        .unwrap();
    assert!(blob.bytes.as_ref().is_empty());
    reopened.close().unwrap();
}
