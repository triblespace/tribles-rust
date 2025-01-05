use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use std::collections::HashSet;
use std::iter::FromIterator;
use sucds::bit_vectors::Rank9Sel;
use sucds::int_vectors::DacsByte;
use sucds::Serializable;
use tribles::blob::schemas::succinctarchive::{
    CachedUniverse, CompressedUniverse, SuccinctArchive, Universe,
};

use tribles::prelude::blobschemas::*;
use tribles::prelude::valueschemas::*;
use tribles::prelude::*;

use tribles::patch::{Entry, IdentityOrder};
use tribles::patch::{SingleSegmentation, PATCH};

use im::OrdSet;

use fake::faker::lorem::en::{Sentence, Words};
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
        "76AE5012877E09FF0EE0868FE9AA0343" as height: R256;
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

fn std_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("std");

    for i in [10, 100, 1000, 10000, 100000, 1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("put", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                HashSet::<Trible>::from_iter(black_box(&samples).iter().copied())
            });
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let set = HashSet::<Trible>::from_iter((&samples).iter().copied());
            b.iter(|| black_box(&set).iter().count());
        });
    }
    //let peak_mem = PEAK_ALLOC.peak_usage_as_gb();
    //println!("The max amount that was used {}", peak_mem);
    group.finish();
}

fn im_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("im");

    for i in [10, 100, 1000, 10000, 100000, 1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("put", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                OrdSet::<Trible>::from_iter(black_box(&samples).iter().copied())
            });
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let set = OrdSet::<Trible>::from_iter(black_box(&samples).iter().copied());
            b.iter(|| black_box(&set).iter().count());
        });
    }
    //let peak_mem = PEAK_ALLOC.peak_usage_as_gb();
    //println!("The max amount that was used {}", peak_mem);
    group.finish();
}

fn patch_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("patch");

    for i in [10, 100, 1000, 10000, 100000, 1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("put", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
                for t in black_box(&samples) {
                    let entry: Entry<64> = Entry::new(&t.data);
                    patch.insert(&entry);
                }
                patch
            })
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
            for t in black_box(&samples) {
                let entry: Entry<64> = Entry::new(&t.data);
                patch.insert(&entry);
            }
            b.iter(|| black_box(&patch).into_iter().count());
        });
        group.bench_with_input(BenchmarkId::new("infixes", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
            for t in black_box(&samples) {
                let entry: Entry<64> = Entry::new(&t.data);
                patch.insert(&entry);
            }
            b.iter(|| {
                let mut i = 0;
                black_box(&patch).infixes(&[0; 0], &mut |_: &[u8; 64]| i += 1);
                i
            });
        });
    }

    let total_unioned = 1000000;
    for i in [2, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(total_unioned as u64));
        group.bench_with_input(BenchmarkId::new("union", i), i, |b, &i| {
            let samples: Vec<Trible> = random_tribles(total_unioned as usize);
            let patchs: Vec<_> = samples
                .chunks(total_unioned / i)
                .map(|samples| {
                    let mut patch: PATCH<64, IdentityOrder, SingleSegmentation> =
                        PATCH::<64, IdentityOrder, SingleSegmentation>::new();
                    for t in samples {
                        let entry: Entry<64> = Entry::new(&t.data);
                        patch.insert(&entry);
                    }
                    patch
                })
                .collect();
            b.iter(|| {
                black_box(&patchs).iter().fold(
                    PATCH::<64, IdentityOrder, SingleSegmentation>::new(),
                    |mut a, p| {
                        a.union(p.clone());
                        a
                    },
                )
            });
        });
    }

    group.finish();
}

fn tribleset_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("tribleset");

    for i in [1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("add", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                //let before_mem = PEAK_ALLOC.current_usage_as_gb();
                let mut set = TribleSet::new();
                for t in black_box(&samples) {
                    set.insert(t);
                }
                //let after_mem = PEAK_ALLOC.current_usage_as_gb();
                //println!("Tribleset size: {}", after_mem - before_mem);
                set
            })
        });
    }

    for i in [1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("from_iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| TribleSet::from_iter(black_box(&samples).iter().copied()))
        });
    }

    group.finish();
}

fn archive_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive");
    group.sample_size(10);

    for i in [1000000] {
        group.throughput(Throughput::Elements(5 * i));
        group.bench_function(BenchmarkId::new("structured/archive", 5 * i), |b| {
            let owner = IdOwner::new();
            let mut set: TribleSet = TribleSet::new();
            (0..i).for_each(|_| {
                let author = owner.defer_insert(fucid());
                let book = owner.defer_insert(fucid());
                set += literature::entity!(&author, {
                    firstname: FirstName(EN).fake::<String>(),
                    lastname: LastName(EN).fake::<String>(),
                });
                set += literature::entity!(&book, {
                    author: &author,
                    title: Words(1..3).fake::<Vec<String>>().join(" "),
                    quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
                });
            });
            b.iter(|| {
                let archive: SuccinctArchive<UNIVERSE, Rank9Sel> = (&set).into();
                let size_domain = archive.domain.size_in_bytes() as f64 / set.len() as f64;
                let size_ae = archive.e_a.size_in_bytes() as f64 / set.len() as f64;
                let size_aa = archive.a_a.size_in_bytes() as f64 / set.len() as f64;
                let size_av = archive.v_a.size_in_bytes() as f64 / set.len() as f64;
                let size_ceav = archive.eav_c.size_in_bytes() as f64 / set.len() as f64;
                let size_cvea = archive.vea_c.size_in_bytes() as f64 / set.len() as f64;
                let size_cave = archive.ave_c.size_in_bytes() as f64 / set.len() as f64;
                let size_cvae = archive.vae_c.size_in_bytes() as f64 / set.len() as f64;
                let size_ceva = archive.eva_c.size_in_bytes() as f64 / set.len() as f64;
                let size_caev = archive.aev_c.size_in_bytes() as f64 / set.len() as f64;
                let size_total = size_domain
                    + size_ae
                    + size_aa
                    + size_av
                    + size_ceav
                    + size_cvea
                    + size_cave
                    + size_cvae
                    + size_ceva
                    + size_caev;

                println!(
                    "Archived trible size: {size_total}\n\
                       Domain:{size_domain}\n\
                       A_e:{size_ae}\n\
                       A_a:{size_aa}\n\
                       A_v:{size_av}\n\
                       C_eav:{size_ceav}\n\
                       C_vea:{size_cvea}\n\
                       C_ave:{size_cave}\n\
                       C_vae:{size_cvae}\n\
                       C_eva:{size_ceva}\n\
                       C_aev:{size_caev}",
                );

                archive
            });
        });
    }

    for i in [1000000] {
        group.throughput(Throughput::Elements(5 * i));
        group.bench_function(BenchmarkId::new("structured/unarchive", 5 * i), |b| {
            let owner = IdOwner::new();
            let mut set: TribleSet = TribleSet::new();
            (0..i).for_each(|_| {
                let author = owner.defer_insert(fucid());
                let book = owner.defer_insert(fucid());
                set += literature::entity!(&author, {
                    firstname: FirstName(EN).fake::<String>(),
                    lastname: LastName(EN).fake::<String>(),
                });
                set += literature::entity!(&book, {
                    author: &author,
                    title: Words(1..3).fake::<Vec<String>>().join(" "),
                    quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
                });
            });
            let archive: SuccinctArchive<UNIVERSE, Rank9Sel> = (&set).into();
            b.iter(|| {
                let set: TribleSet = (&archive).into();
                set
            });
        });
    }

    for i in [1000000] {
        group.throughput(Throughput::Elements(i));
        group.bench_with_input(BenchmarkId::new("random/archive", i), &i, |b, &i| {
            let samples = random_tribles(i as usize);
            let set = TribleSet::from_iter(black_box(&samples).iter().copied());
            b.iter(|| {
                let archive: SuccinctArchive<UNIVERSE, Rank9Sel> = (&set).into();
                let size_domain = archive.domain.size_in_bytes() as f64 / set.len() as f64;
                let size_ae = archive.e_a.size_in_bytes() as f64 / set.len() as f64;
                let size_aa = archive.a_a.size_in_bytes() as f64 / set.len() as f64;
                let size_av = archive.v_a.size_in_bytes() as f64 / set.len() as f64;
                let size_ceav = archive.eav_c.size_in_bytes() as f64 / set.len() as f64;
                let size_cvea = archive.vea_c.size_in_bytes() as f64 / set.len() as f64;
                let size_cave = archive.ave_c.size_in_bytes() as f64 / set.len() as f64;
                let size_cvae = archive.vae_c.size_in_bytes() as f64 / set.len() as f64;
                let size_ceva = archive.eva_c.size_in_bytes() as f64 / set.len() as f64;
                let size_caev = archive.aev_c.size_in_bytes() as f64 / set.len() as f64;
                let size_total = size_domain
                    + size_ae
                    + size_aa
                    + size_av
                    + size_ceav
                    + size_cvea
                    + size_cave
                    + size_cvae
                    + size_ceva
                    + size_caev;

                println!(
                    "Archived trible size: {size_total}\n\
                       Domain:{size_domain}\n\
                       A_e:{size_ae}\n\
                       A_a:{size_aa}\n\
                       A_v:{size_av}\n\
                       C_eav:{size_ceav}\n\
                       C_vea:{size_cvea}\n\
                       C_ave:{size_cave}\n\
                       C_vae:{size_cvae}\n\
                       C_eva:{size_ceva}\n\
                       C_aev:{size_caev}",
                );

                archive
            });
        });
    }

    for i in [1000000] {
        group.throughput(Throughput::Elements(i));
        group.bench_with_input(BenchmarkId::new("random/unarchive", i), &i, |b, &i| {
            let samples = random_tribles(i as usize);
            let set = TribleSet::from_iter(black_box(&samples).iter().copied());
            let archive: SuccinctArchive<UNIVERSE, Rank9Sel> = (&set).into();
            b.iter(|| {
                let set: TribleSet = (&archive).into();
                set
            });
        });
    }

    group.finish();
}

fn entities_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("entities");

    group.throughput(Throughput::Elements(5));
    group.bench_function(BenchmarkId::new("entities", 5), |b| {
        b.iter(|| {
            let owner = IdOwner::new();
            let mut kb = TribleSet::new();
            {
                let author = owner.defer_insert(fucid());
                let book = owner.defer_insert(fucid());
                kb += literature::entity!(&author, {
                    firstname: FirstName(EN).fake::<String>(),
                    lastname: LastName(EN).fake::<String>(),
                });
                kb += literature::entity!(&book, {
                    author: &author,
                    title: Words(1..3).fake::<Vec<String>>().join(" "),
                    quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
                });
            }
            (kb, owner)
        })
    });

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(5 * i));
        group.bench_function(BenchmarkId::new("union", 5 * i), |b| {
            b.iter(|| {
                let kb = (0..i)
                    .flat_map(|_| {
                        let owner = IdOwner::new();
                        let author = owner.defer_insert(fucid());
                        let book = owner.defer_insert(fucid());
                        [
                            literature::entity!(&author, {
                                firstname: FirstName(EN).fake::<String>(),
                                lastname: LastName(EN).fake::<String>(),
                            }),
                            literature::entity!(&book, {
                                author: &author,
                                title: Words(1..3).fake::<Vec<String>>().join(" "),
                                quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
                            }),
                        ]
                    })
                    .fold(TribleSet::new(), |kb, set| kb + set);
                kb
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(5 * i));
        group.bench_function(BenchmarkId::new("union/prealloc", 5 * i), |b| {
            let sets: Vec<_> = (0..i)
                .flat_map(|_| {
                    let owner = IdOwner::new();
                    let author = owner.defer_insert(fucid());
                    let book = owner.defer_insert(fucid());

                    [
                        literature::entity!(&author, {
                            firstname: FirstName(EN).fake::<String>(),
                            lastname: LastName(EN).fake::<String>(),
                        }),
                        literature::entity!(&book, {
                            author: &author,
                            title: Words(1..3).fake::<Vec<String>>().join(" "),
                            quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
                        }),
                    ]
                })
                .collect();
            b.iter(|| {
                let mut kb = TribleSet::new();
                for set in &sets {
                    kb += set.clone();
                }
                kb
            });
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(5 * i));
        group.bench_function(BenchmarkId::new("union/parallel", 5 * i), |b| {
            b.iter(|| {
                let kb = (0..i)
                    .into_par_iter()
                    .flat_map(|_| {
                        let owner = IdOwner::new();
                        let author = owner.defer_insert(fucid());
                        let book = owner.defer_insert(fucid());

                        [
                            literature::entity!(&author, {
                                firstname: FirstName(EN).fake::<String>(),
                                lastname: LastName(EN).fake::<String>(),
                            }),
                            literature::entity!(&book, {
                                author: &author,
                                title: Words(1..3).fake::<Vec<String>>().join(" "),
                                quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
                            }),
                        ]
                    })
                    .reduce(|| TribleSet::new(), |a, b| a + b);
                kb
            })
        });
    }

    let total_unioned = 1000000;
    for i in [2, 10, 100, 1000] {
        group.throughput(Throughput::Elements(5 * total_unioned as u64));
        group.bench_with_input(
            BenchmarkId::new("union/parallel/chunked", i),
            &i,
            |b, &i| {
                let subsets: Vec<TribleSet> = (0..i)
                    .into_par_iter()
                    .map(|_| {
                        (0..total_unioned / i)
                            .flat_map(|_| {
                                let owner = IdOwner::new();
                                let author = owner.defer_insert(fucid());
                                let book = owner.defer_insert(fucid());
                                [
                                    literature::entity!(&author, {
                                        firstname: FirstName(EN).fake::<String>(),
                                        lastname: LastName(EN).fake::<String>(),
                                    }),
                                    literature::entity!(&book, {
                                        author: &author,
                                        title: Words(1..3).fake::<Vec<String>>().join(" "),
                                        quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
                                    }),
                                ]
                            })
                            .fold(TribleSet::new(), |kb, set| kb + set)
                    })
                    .collect();
                b.iter(|| {
                    subsets
                        .iter()
                        .cloned()
                        .fold(TribleSet::new(), |kb, set| kb + set)
                });
            },
        );
    }

    group.finish();
}

fn query_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");

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
            quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
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
                has gone there will be nothing. Only I will remain.".to_blob().as_handle()
    });

    (0..1000).for_each(|_| {
        let author = owner.defer_insert(fucid());
        let book = owner.defer_insert(fucid());
        kb += literature::entity!(&author, {
            firstname: "Fake",
            lastname: "Herbert",
        });
        kb += literature::entity!(&book, {
            author: &author,
            title: Words(1..3).fake::<Vec<String>>().join(" "),
            quote: Sentence(5..25).fake::<String>().to_blob().as_handle()
        });
    });

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("tribleset/single", 1), |b| {
        b.iter(|| {
            find!(
            (author: Value<_>, title: Value<_>, quote: Value<_>),
            literature::pattern!(&kb, [
            {author @
                firstname: ("Frank"),
                lastname: ("Herbert")},
            { author: author,
              title: title,
              quote: quote
            }]))
            .count()
        })
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function(BenchmarkId::new("tribleset/multi", 1000), |b| {
        b.iter(|| {
            find!(
            (author: Value<_>, title: Value<_>, quote: Value<_>),
            literature::pattern!(&kb, [
            {author @
                firstname: (black_box("Fake")),
                lastname: (black_box("Herbert"))},
            { author: author,
              title: title,
              quote: quote
            }]))
            .count()
        })
    });

    group.sample_size(10);

    let kb_archive: SuccinctArchive<UNIVERSE, Rank9Sel> = (&kb).into();

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("archive/single", 1), |b| {
        b.iter(|| {
            find!(
            (author: Value<_>, title: Value<_>, quote: Value<_>),
            literature::pattern!(&kb_archive, [
            {author @
                firstname: (black_box("Frank")),
                lastname: (black_box("Herbert"))},
            { author: author,
              title: title,
              quote: quote
            }]))
            .count()
        })
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function(BenchmarkId::new("archive/multi", 1000), |b| {
        b.iter(|| {
            find!(
            (author: Value<_>, title: Value<_>, quote: Value<_>),
            literature::pattern!(&kb_archive, [
            {author @
                firstname: (black_box("Fake")),
                lastname: (black_box("Herbert"))},
            { author: author,
              title: title,
              quote: quote
            }]))
            .count()
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    //std_benchmark,
    //im_benchmark,

    patch_benchmark,
    tribleset_benchmark,
    archive_benchmark,
    entities_benchmark,
    query_benchmark,
);

criterion_main!(benches);
