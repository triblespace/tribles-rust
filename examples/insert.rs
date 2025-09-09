use crate::entity;
use crate::path;
use crate::pattern;
use crate::pattern_changes;
use fake::faker::lorem::en::Sentence;
use fake::faker::lorem::en::Words;
use tribles::prelude::blobschemas::*;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

use tribles::examples::literature;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;
use tribles::repo::BlobStorePut;

use blobschemas::LongString;
use valueschemas::GenId;
use valueschemas::Blake3;
use valueschemas::Handle;
use valueschemas::R256;
use valueschemas::ShortString;

fn main() {
    let mut kb = TribleSet::new();
    let mut blobs = MemoryBlobStore::new();
    (0..1000000).for_each(|_| {
        let author = fucid();
        let book = fucid();
        kb += entity!(&author, {
            literature::firstname: FirstName(EN).fake::<String>(),
            literature::lastname: LastName(EN).fake::<String>(),
        });
        kb += entity!(&book, {
            literature::author: &author,
            literature::title: Words(1..3).fake::<Vec<String>>().join(" "),
            literature::quote: blobs.put(Sentence(5..25).fake::<String>()).unwrap()
        });
    });
}
