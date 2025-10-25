use std::fs::OpenOptions;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use triblespace::blob::schemas::UnknownBlob;
use triblespace::blob::Blob;
use triblespace::blob::Bytes;
use triblespace::repo::pile::GetBlobError;
use triblespace::repo::pile::Pile;
use triblespace::repo::BlobStore;
use triblespace::repo::BlobStorePut;

// size of the blob header in the pile format
const BLOB_HEADER_LEN: u64 = 16 + 8 + 8 + 32;

#[test]
fn iterator_errors_on_corrupt_blob() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pile.pile");

    {
        let mut pile: Pile = Pile::open(&path).unwrap();
        let blob: Blob<UnknownBlob> = Blob::new(Bytes::from_source(b"hello world".as_slice()));
        pile.put(blob).unwrap();
        pile.flush().unwrap();
        pile.close().unwrap();
    }

    // Corrupt the blob payload
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.seek(SeekFrom::Start(BLOB_HEADER_LEN)).unwrap();
    file.write_all(&[0]).unwrap();
    file.flush().unwrap();

    let mut pile: Pile = Pile::open(&path).unwrap();
    pile.restore().unwrap();
    let reader = pile.reader().unwrap();
    let mut iter = reader.iter();
    match iter.next() {
        Some(Err(GetBlobError::ValidationError(_))) => {}
        other => panic!("expected validation error, got {:?}", other),
    }
    pile.close().unwrap();
}
