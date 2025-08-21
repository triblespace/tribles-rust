#![cfg(feature = "succinct-archive")]

use criterion::criterion_group;
use criterion::criterion_main;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use jerky::bit_vector::rank9sel::Rank9SelIndex;
use jerky::int_vectors::DacsByte;
use rand::thread_rng;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashSet;
use std::hint::black_box;
use std::iter::FromIterator;
use tribles::blob::schemas::succinctarchive::CachedUniverse;
use tribles::blob::schemas::succinctarchive::CompressedUniverse;
use tribles::blob::schemas::succinctarchive::SuccinctArchive;
use tribles::blob::schemas::succinctarchive::Universe;
use tribles::blob::schemas::UnknownBlob;
use tribles::repo::BlobStorePut;

use tribles::prelude::blobschemas::*;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

use tribles::patch::Entry;
use tribles::patch::IdentityOrder;
use tribles::patch::PATCH;

use im::OrdSet;

use fake::faker::lorem::en::Sentence;
use fake::faker::lorem::en::Words;
use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

type UNIVERSE = CachedUniverse<1_048_576, 1_048_576, CompressedUniverse<DacsByte>>;

//use peak_alloc::PeakAlloc;
//#[global_allocator]
//static PEAK_ALLOC: PeakAlloc = PeakAlloc;

NS! {
    pub namespace literature {
        "8F180883F9FD5F787E9E0AF0DF5866B9" as author: GenId;
        "0DBB530B37B966D137C50B943700EDB2" as firstname: ShortString;
        "6BAA463FD4EAF45F6A103DB9433E4545" as lastname: ShortString;
        "A74AA63539354CDA47F387A4C3A8D54C" as title: ShortString;
        "FCCE870BECA333D059D5CD68C43B98F0" as page_count: R256;
        "6A03BAF6CFB822F04DA164ADAAEB53F6" as quote: Handle<Blake3, LongString>;
    }
}

fn random_tribles(length: usize) -> Vec<Trible> {
    let owner = IdOwner::new();
    let mut rng = thread_rng();

    let mut vec = Vec::new();

    let mut e = owner.defer_insert(fucid());
    let mut a = owner.defer_insert(fucid());

    for _i in 0..length {
        if rng.gen_bool(0.5) {
            e = owner.defer_insert(fucid());
        }
        if rng.gen_bool(0.5) {
            a = owner.defer_insert(fucid());
        }

        let v = fucid();
        vec.push(Trible::new(&e, &a, &v.to_value()))
    }
    return vec;
}

fn main() {
    let owner = IdOwner::new();
    let mut kb = TribleSet::new();
    (0..1000000).for_each(|_| {
        let author = owner.defer_insert(fucid());
        let book = owner.defer_insert(fucid());
        kb += literature::entity!(&author, {
            firstname: FirstName(EN).fake::<String>(),
            lastname: LastName(EN).fake::<String>(),
        });
        kb += literature::entity!(&book, {
            author: &author,
            title: Words(1..3).fake::<Vec<String>>().join(" "),
            quote: Sentence(5..25).fake::<String>().to_blob().get_handle()
        });
    });

    let author = owner.defer_insert(fucid());
    let book = owner.defer_insert(fucid());
    kb += literature::entity!(&author, {
        firstname: "Frank",
        lastname: "Herbert",
    });
    kb += literature::entity!(&book, {
        author: &author,
        title: "Dune",
        quote: "I must not fear. Fear is the \
                mind-killer. Fear is the little-death that brings total \
                obliteration. I will face my fear. I will permit it to \
                pass over me and through me. And when it has gone past I \
                will turn the inner eye to see its path. Where the fear \
                has gone there will be nothing. Only I will remain.".to_blob().get_handle()
    });

    let fanks = find!(
        (author: Value<_>),
        literature::pattern!(&kb, [
        {author @ firstname: ("Frank")}]))
    .count();

    let herberts = find!(
        (author: Value<_>),
        literature::pattern!(&kb, [
        {author @ lastname: ("Herbert")}]))
    .count();

    println!("Found {} authors named Frank", fanks);
    println!("Found {} authors with the last name Herbert", herberts);

    (0..1000000).for_each(|_| {
        let count = find!(
        (author: Value<_>, title: Value<_>, quote: Value<_>),
        literature::pattern!(&kb, [
        {author @
            firstname: ("Frank"),
            lastname: ("Herbert")},
        { author: author,
          title: title,
          quote: quote
        }]))
        .count();
    });
}
