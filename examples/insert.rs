use crate::entity;
use crate::path;
use crate::pattern;
use crate::pattern_changes;
use fake::faker::lorem::en::Sentence;
use fake::faker::lorem::en::Words;
use tribles::prelude::blobschemas::*;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

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

hey 

fields! {
    "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
    "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
    "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
    "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
    "FCCE870BECA333D059D5CD68C43B98F0" as page_count: R256;
    "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
}

fn main() {
    let mut kb = TribleSet::new();
    let mut blobs = MemoryBlobStore::new();
    (0..1000000).for_each(|_| {
        let author_ = fucid();
        let book = fucid();
        kb += entity!(&author_, {
            firstname: FirstName(EN).fake::<String>(),
            lastname: LastName(EN).fake::<String>(),
        });
        kb += entity!(&book, {
            author: &author_,
            title: Words(1..3).fake::<Vec<String>>().join(" "),
            quote: blobs.put(Sentence(5..25).fake::<String>()).unwrap()
        });
    });
}
