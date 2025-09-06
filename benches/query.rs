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
use crate::pattern;
use crate::entity;
use crate::pattern_changes;
use crate::path;


use tribles::prelude::blobschemas::*;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

use tribles::patch::Entry;
use tribles::patch::IdentitySchema;
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

pub mod literature {
    #![allow(unused)]
    use crate::prelude::*;
    use crate::value::schemas::hash::Handle;
    use crate::value::schemas::hash::Blake3;
    use crate::blob::schemas::longstring::LongString;
    use crate::value::schemas::shortstring::ShortString;
    use crate::value::schemas::genid::GenId;
    use crate::value::schemas::r256::R256;
    pub const author: crate::field::Field<GenId> = crate::field::Field::from(hex_literal::hex!("8F180883F9FD5F787E9E0AF0DF5866B9"));
    pub const firstname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("0DBB530B37B966D137C50B943700EDB2"));
    pub const lastname: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("6BAA463FD4EAF45F6A103DB9433E4545"));
    pub const title: crate::field::Field<ShortString> = crate::field::Field::from(hex_literal::hex!("A74AA63539354CDA47F387A4C3A8D54C"));
    pub const page_count: crate::field::Field<R256> = crate::field::Field::from(hex_literal::hex!("FCCE870BECA333D059D5CD68C43B98F0"));
    pub const quote: crate::field::Field<Handle<Blake3, LongString>> = crate::field::Field::from(hex_literal::hex!("6A03BAF6CFB822F04DA164ADAAEB53F6"));
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
        kb += entity!(&author, {
            literature::firstname: FirstName(EN).fake::<String>(),
            literature::lastname: LastName(EN).fake::<String>(),
        });
        kb += entity!(&book, {
            literature::author: &author,
            literature::title: Words(1..3).fake::<Vec<String>>().join(" "),
            literature::quote: Sentence(5..25).fake::<String>().to_blob().get_handle()
        });
    });

    let author = owner.defer_insert(fucid());
    let book = owner.defer_insert(fucid());
    kb += entity!(&author, {
        literature::firstname: "Frank",
        literature::lastname: "Herbert",
    });
    kb += entity!(&book, {
        literature::author: &author,
        literature::title: "Dune",
        literature::quote: "I must not fear. Fear is the \
                mind-killer. Fear is the little-death that brings total \
                obliteration. I will face my fear. I will permit it to \
                pass over me and through me. And when it has gone past I \
                will turn the inner eye to see its path. Where the fear \
                has gone there will be nothing. Only I will remain.".to_blob().get_handle()
    });

    let fanks = find!(
        (author: Value<_>),
        pattern!(&kb, [
        {author @ literature::firstname: ("Frank")}]))
    .count();

    let herberts = find!(
        (author: Value<_>),
        pattern!(&kb, [
        {author @ literature::lastname: ("Herbert")}]))
    .count();

    println!("Found {} authors named Frank", fanks);
    println!("Found {} authors with the last name Herbert", herberts);

    (0..1000000).for_each(|_| {
        let count = find!(
        (author: Value<_>, title: Value<_>, quote: Value<_>),
        pattern!(&kb, [
        {author @
            literature::firstname: ("Frank"),
            literature::lastname: ("Herbert")},
        { literature::author: author,
          literature::title: title,
          literature::quote: quote
        }]))
        .count();
    });
}
