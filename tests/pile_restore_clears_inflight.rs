use anybytes::Bytes;
use std::fs::OpenOptions;
use tempfile;
use tribles::blob::schemas::UnknownBlob;
use tribles::prelude::*;

#[test]
fn restore_clears_inflight_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pile.pile");

    let mut pile: Pile = Pile::open(&path).unwrap();
    let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(b"blob".to_vec()));
    let handle = pile.put(blob.clone()).unwrap();
    pile.flush().unwrap();

    let len = std::fs::metadata(&path).unwrap().len();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(len - 1)
        .unwrap();

    pile.restore().unwrap();

    pile.put(blob).unwrap();
    pile.flush().unwrap();
    pile.refresh().unwrap();

    let stored = pile
        .reader()
        .unwrap()
        .get::<Blob<UnknownBlob>, _>(handle)
        .unwrap();
    assert_eq!(stored.bytes.as_ref(), b"blob");
}
