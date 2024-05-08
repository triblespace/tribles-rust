use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use hex::ToHex;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use sucds::Serializable;
use std::collections::HashSet;
use std::convert::TryInto;
use std::iter::FromIterator;
use tribles::{and, TribleSetArchive};
use tribles::transient::Transient;
use tribles::NS;
use tribles::{types::SmallString, Id};

use tribles::test::hashtribleset::HashTribleSet;
use tribles::ufoid;
use tribles::{find, trible::*};

use tribles::patch::{Entry, IdentityOrder};
use tribles::patch::{SingleSegmentation, PATCH};
use tribles::TribleSet;

use im::OrdSet;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

use peak_alloc::PeakAlloc;
#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

NS! {
    pub namespace knights {
        "39E2D06DBCD9CB96DE5BC46F362CFF31" as loves: tribles::Id;
        "7D4F339CC4AE0BBA2765F34BE1D108EF" as name: tribles::types::SmallString;
        "3E0C58AC884072EA6429BB00A1BA1DA4" as title: tribles::types::SmallString;
    }
}

fn random_tribles(length: usize) -> Vec<Trible> {
    let mut rng = thread_rng();

    let mut vec = Vec::new();

    let mut e = ufoid();
    let mut a = ufoid();

    for _i in 0..length {
        if rng.gen_bool(0.5) {
            e = ufoid();
        }
        if rng.gen_bool(0.5) {
            a = ufoid();
        }

        let v = ufoid();
        vec.push(Trible::new(e, a, v))
    }
    return vec;
}

fn std_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("std");

    for i in [10, 100, 1000, 10000, 100000, 1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("put", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter_with_large_drop(|| {
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

fn hashtribleset_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashtribleset");

    for i in [1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("add", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter_with_large_drop(|| {
                let before_mem = PEAK_ALLOC.current_usage_as_gb();
                let mut set = HashTribleSet::new();
                for t in black_box(&samples) {
                    set.insert(t);
                }
                let after_mem = PEAK_ALLOC.current_usage_as_gb();
                println!("HashTribleset size: {}", after_mem - before_mem);
                set
            })
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
            b.iter_with_large_drop(|| {
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
            b.iter_with_large_drop(|| {
                let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation, ()>::new();
                for t in black_box(&samples) {
                    let entry: Entry<64, ()> = Entry::new(&t.data, ());
                    patch.insert(&entry);
                }
                patch
            })
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation, ()>::new();
            for t in black_box(&samples) {
                let entry: Entry<64, ()> = Entry::new(&t.data, ());
                patch.insert(&entry);
            }
            b.iter(|| black_box(&patch).into_iter().count());
        });
        group.bench_with_input(BenchmarkId::new("infixes", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation, ()>::new();
            for t in black_box(&samples) {
                let entry: Entry<64, ()> = Entry::new(&t.data, ());
                patch.insert(&entry);
            }
            b.iter(|| {
                let mut i = 0;
                black_box(&patch).infixes(&[0; 0], &mut |_: [u8; 64]| i += 1);
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
                    let mut patch: PATCH<64, IdentityOrder, SingleSegmentation, ()> =
                        PATCH::<64, IdentityOrder, SingleSegmentation, ()>::new();
                    for t in samples {
                        let entry: Entry<64, ()> = Entry::new(&t.data, ());
                        patch.insert(&entry);
                    }
                    patch
                })
                .collect();
            b.iter_with_large_drop(|| {
                black_box(&patchs).iter().fold(
                    PATCH::<64, IdentityOrder, SingleSegmentation, ()>::new(),
                    |mut a, p| {
                        a.union(p);
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
            b.iter_with_large_drop(|| {
                let before_mem = PEAK_ALLOC.current_usage_as_gb();
                let mut set = TribleSet::new();
                for t in black_box(&samples) {
                    set.insert(t);
                }
                let after_mem = PEAK_ALLOC.current_usage_as_gb();
                println!("Tribleset size: {}", after_mem - before_mem);
                set
            })
        });
    }

    for i in [1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("from_iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter_with_large_drop(|| TribleSet::from_iter(black_box(&samples).iter().copied()))
        });
    }

    group.finish();
}

fn archive_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive");
    group.sample_size(10);

    for i in [1000000] {
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("structured", 4 * i), |b| {
            let mut set: TribleSet = TribleSet::new();
            (0..i).for_each(|_| {
                let lover_a = ufoid();
                let lover_b = ufoid();
                knights::entity!(&mut set, lover_a, {
                    name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                    loves: lover_b
                });
                knights::entity!(&mut set, lover_b, {
                    name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                    loves: lover_a
                });
            });
            b.iter_with_large_drop(|| {
                let archive = TribleSetArchive::with(&set);
                println!("Archived trible size:");
                println!("  Domain:{}", (archive.domain.len()*32) as f64 / set.len() as f64);
                println!("  A_e:{}", archive.e_a.size_in_bytes() as f64 / set.len() as f64);
                println!("  A_a:{}", archive.a_a.size_in_bytes() as f64 / set.len() as f64);
                println!("  A_v:{}", archive.v_a.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_eav:{}", archive.eav_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_vea:{}", archive.vea_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_ave:{}", archive.ave_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_vae:{}", archive.vae_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_eva:{}", archive.eva_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_aev:{}", archive.aev_c.size_in_bytes() as f64 / set.len() as f64);

                archive
            });
        });
    }

    for i in [1000000] {
        group.throughput(Throughput::Elements(i));
        group.bench_with_input(BenchmarkId::new("random", i), &i, |b, &i| {
            let samples = random_tribles(i as usize);
            let set = TribleSet::from_iter(black_box(&samples).iter().copied());
            b.iter_with_large_drop(|| {
                let archive = TribleSetArchive::with(&set);
                println!("Archived trible size:");
                println!("  Domain:{}", (archive.domain.len()*32) as f64 / set.len() as f64);
                println!("  A_e:{}", archive.e_a.size_in_bytes() as f64 / set.len() as f64);
                println!("  A_a:{}", archive.a_a.size_in_bytes() as f64 / set.len() as f64);
                println!("  A_v:{}", archive.v_a.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_eav:{}", archive.eav_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_vea:{}", archive.vea_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_ave:{}", archive.ave_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_vae:{}", archive.vae_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_eva:{}", archive.eva_c.size_in_bytes() as f64 / set.len() as f64);
                println!("  C_aev:{}", archive.aev_c.size_in_bytes() as f64 / set.len() as f64);

                archive
            });
        });
    }

    group.finish();
}

fn entities_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("entities");

    group.throughput(Throughput::Elements(4));
    group.bench_function(BenchmarkId::new("entities", 4), |b| {
        b.iter_with_large_drop(|| {
            let mut kb = TribleSet::new();
            let lover_a = ufoid();
            let lover_b = ufoid();

            kb.union(&knights::entity!(lover_a, {
                name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                loves: lover_b
            }));
            kb.union(&knights::entity!(lover_b, {
                name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                loves: lover_a
            }));

            kb
        })
    });

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("direct", 4 * i), |b| {
            b.iter_with_large_drop(|| {
                let before_mem = PEAK_ALLOC.current_usage();
                let mut kb: TribleSet = TribleSet::new();
                (0..i).for_each(|_| {
                    let lover_a = ufoid();
                    let lover_b = ufoid();
                    knights::entity!(&mut kb, lover_a, {
                        name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                        loves: lover_b
                    });
                    knights::entity!(&mut kb, lover_b, {
                        name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                        loves: lover_a
                    });
                });
                let after_mem = PEAK_ALLOC.current_usage();
                println!(
                    "Trible size: {}",
                    (after_mem - before_mem) / kb.len() as usize
                );
                kb
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union", 4 * i), |b| {
            b.iter_with_large_drop(|| {
                let kb = (0..i)
                    .flat_map(|_| {
                        let lover_a = ufoid();
                        let lover_b = ufoid();

                        [
                            knights::entity!(lover_a, {
                                name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                                loves: lover_b
                            }),
                            knights::entity!(lover_b, {
                                name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                                loves: lover_a
                            }),
                        ]
                    })
                    .fold(TribleSet::new(), |mut kb, set| {
                        kb.union(&set);
                        kb
                    });
                kb
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union/prealloc", 4 * i), |b| {
            let sets: Vec<_> = (0..i)
                .flat_map(|_| {
                    let lover_a = ufoid();
                    let lover_b = ufoid();

                    [
                        knights::entity!(lover_a, {
                            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                            loves: lover_b
                        }),
                        knights::entity!(lover_b, {
                            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                            loves: lover_a
                        }),
                    ]
                })
                .collect();
            b.iter_with_large_drop(|| {
                let mut kb = TribleSet::new();
                for set in &sets {
                    kb.union(&set);
                }
                kb
            });
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union/parallel", 4 * i), |b| {
            b.iter_with_large_drop(|| {
                let kb = (0..i)
                    .into_par_iter()
                    .flat_map(|_| {
                        let lover_a = ufoid();
                        let lover_b = ufoid();

                        [
                            knights::entity!(lover_a, {
                                name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                                loves: lover_b
                            }),
                            knights::entity!(lover_b, {
                                name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                                loves: lover_a
                            }),
                        ]
                    })
                    .reduce(
                        || TribleSet::new(),
                        |mut a, b| {
                            a.union(&b);
                            a
                        },
                    );
                kb
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union/parallel/prealloc", 4 * i), |b| {
            let sets: Vec<_> = (0..i)
                .flat_map(|_| {
                    let lover_a = ufoid();
                    let lover_b = ufoid();

                    [
                        knights::entity!(lover_a, {
                            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                            loves: lover_b
                        }),
                        knights::entity!(lover_b, {
                            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                            loves: lover_a
                        }),
                    ]
                })
                .collect();
            b.iter_with_large_drop(|| {
                let kb = sets.par_iter().cloned().reduce(
                    || TribleSet::new(),
                    |mut a, b| {
                        a.union(&b);
                        a
                    },
                );
                kb
            });
        });
    }

    for i in [1000000] {
        let batch_size = 2;
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union/parallel/chunked", 4 * i), |b| {
            b.iter_with_large_drop(|| {
                let kb = (0..batch_size)
                    .into_par_iter()
                    .map(|_| {
                        (0..i / batch_size)
                            .flat_map(|_| {
                                let lover_a = ufoid();
                                let lover_b = ufoid();

                                [
                                    knights::entity!(lover_a, {
                                        name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                                        loves: lover_b
                                    }),
                                    knights::entity!(lover_b, {
                                        name: Name(EN).fake::<String>()[..].try_into().unwrap(),
                                        loves: lover_a
                                    }),
                                ]
                            })
                            .fold(TribleSet::new(), |mut kb, set| {
                                kb.union(&set);
                                kb
                            })
                    })
                    .reduce(
                        || TribleSet::new(),
                        |mut a, b| {
                            a.union(&b);
                            a
                        },
                    );
                kb
            })
        });
    }

    group.finish();
}

fn query_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");

    let mut kb = TribleSet::new();
    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();

        kb.union(&knights::entity!(lover_a, {
            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
            loves: lover_b
        }));
        kb.union(&knights::entity!(lover_b, {
            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
            loves: lover_a
        }));
    });

    let mut data_kb = TribleSet::new();

    let juliet = ufoid();
    let romeo = ufoid();

    kb.union(&knights::entity!(juliet, {
        name: "Juliet".try_into().unwrap(),
        loves: romeo
    }));
    kb.union(&knights::entity!(romeo, {
        name: "Romeo".try_into().unwrap(),
        loves: juliet
    }));

    (0..1000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();

        data_kb.union(&knights::entity!(lover_a, {
            name: "Wameo".try_into().unwrap(),
            loves: lover_b
        }));
        data_kb.union(&knights::entity!(lover_b, {
            name: Name(EN).fake::<String>()[..].try_into().unwrap(),
            loves: lover_a
        }));
    });

    kb.union(&data_kb);

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("pattern", 1), |b| {
        b.iter(|| {
            find!(
                ctx,
                (juliet, name),
                knights::pattern!(ctx, kb, [
                {name: (black_box("Romeo").try_into().unwrap()),
                 loves: juliet},
                {juliet @
                    name: name
                }])
            )
            .count()
        })
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function(BenchmarkId::new("pattern", 1000), |b| {
        b.iter(|| {
            find!(
                ctx,
                (juliet, name),
                knights::pattern!(ctx, kb, [
                {name: (black_box("Wameo").try_into().unwrap()),
                 loves: juliet},
                {juliet @
                    name: name
                }])
            )
            .count()
        })
    });
    group.finish();
}

fn attribute_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("attribute");

    let mut name: Transient<SmallString> = Transient::new();
    let mut loves: Transient<Id> = Transient::new();

    (0..1000000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(
            &lover_a,
            &(Name(EN).fake::<String>()[..].try_into().unwrap()),
        );
        name.insert(
            &lover_b,
            &(Name(EN).fake::<String>()[..].try_into().unwrap()),
        );
        loves.insert(&lover_a, &lover_b);
        loves.insert(&lover_b, &lover_a);
    });

    (0..1000).for_each(|_| {
        let lover_a = ufoid();
        let lover_b = ufoid();
        name.insert(&lover_a, &("Wameo".try_into().unwrap()));
        name.insert(
            &lover_b,
            &(Name(EN).fake::<String>()[..].try_into().unwrap()),
        );
        loves.insert(&lover_a, &lover_b);
        loves.insert(&lover_b, &lover_a);
    });

    let romeo = ufoid();
    let juliet = ufoid();
    name.insert(&romeo, &("Romeo".try_into().unwrap()));
    name.insert(&juliet, &("Juliet".try_into().unwrap()));
    loves.insert(&romeo, &juliet);
    loves.insert(&juliet, &romeo);

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("query", 1), |b| {
        b.iter(|| {
            find!(
                ctx,
                (juliet, romeo, romeo_name, juliet_name),
                and!(
                    romeo_name.is(black_box("Romeo").try_into().unwrap()),
                    name.has(romeo, romeo_name),
                    name.has(juliet, juliet_name),
                    loves.has(romeo, juliet)
                )
            )
            .count()
        })
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function(BenchmarkId::new("query", 1000), |b| {
        b.iter(|| {
            find!(
                ctx,
                (juliet, romeo, romeo_name, juliet_name),
                and!(
                    romeo_name.is(black_box("Wameo").try_into().unwrap()),
                    name.has(romeo, romeo_name),
                    name.has(juliet, juliet_name),
                    loves.has(romeo, juliet)
                )
            )
            .count()
        })
    });
    group.finish();
}

fn oxigraph_benchmark(c: &mut Criterion) {
    use oxigraph::model::*;
    use oxigraph::sparql::QueryResults;
    use oxigraph::store::Store;

    let loves_node =
        NamedNode::new(["urn:id:", &knights::ids::loves.encode_hex_upper::<String>()].concat())
            .unwrap();
    let name_node =
        NamedNode::new(["urn:id:", &knights::ids::name.encode_hex_upper::<String>()].concat())
            .unwrap();

    let mut group = c.benchmark_group("oxigraph");

    //insert
    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("insert dataset", 4 * i), |b| {
            b.iter_with_large_drop(|| {
                let before_mem = PEAK_ALLOC.current_usage();

                let mut dataset = Dataset::default();
                (0..i).for_each(|_| {
                    let lover_a =
                        NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat())
                            .unwrap();
                    let lover_b =
                        NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat())
                            .unwrap();

                    let quad = Quad::new(
                        lover_a.clone(),
                        loves_node.clone(),
                        lover_b.clone(),
                        GraphName::DefaultGraph,
                    );
                    dataset.insert(&quad);

                    let quad = Quad::new(
                        lover_b.clone(),
                        loves_node.clone(),
                        lover_a.clone(),
                        GraphName::DefaultGraph,
                    );
                    dataset.insert(&quad);

                    let name = Literal::new_simple_literal(Name(EN).fake::<String>());
                    let quad = Quad::new(
                        lover_a.clone(),
                        name_node.clone(),
                        name,
                        GraphName::DefaultGraph,
                    );
                    dataset.insert(&quad);

                    let name = Literal::new_simple_literal(Name(EN).fake::<String>());
                    let quad = Quad::new(
                        lover_b.clone(),
                        name_node.clone(),
                        name,
                        GraphName::DefaultGraph,
                    );
                    dataset.insert(&quad);
                });
                let after_mem = PEAK_ALLOC.current_usage();
                println!(
                    "Quad size: {}",
                    (after_mem - before_mem) / dataset.len() as usize
                );
                dataset
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("insert store", 4 * i), |b| {
            b.iter_with_large_drop(|| {
                let before_mem = PEAK_ALLOC.current_usage();

                let store = Store::new().unwrap();
                (0..i).for_each(|_| {
                    let lover_a =
                        NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat())
                            .unwrap();
                    let lover_b =
                        NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat())
                            .unwrap();

                    let quad = Quad::new(
                        lover_a.clone(),
                        loves_node.clone(),
                        lover_b.clone(),
                        GraphName::DefaultGraph,
                    );
                    store.insert(&quad).unwrap();

                    let quad = Quad::new(
                        lover_b.clone(),
                        loves_node.clone(),
                        lover_a.clone(),
                        GraphName::DefaultGraph,
                    );
                    store.insert(&quad).unwrap();

                    let name = Literal::new_simple_literal(Name(EN).fake::<String>());
                    let quad = Quad::new(
                        lover_a.clone(),
                        name_node.clone(),
                        name,
                        GraphName::DefaultGraph,
                    );
                    store.insert(&quad).unwrap();

                    let name = Literal::new_simple_literal(Name(EN).fake::<String>());
                    let quad = Quad::new(
                        lover_b.clone(),
                        name_node.clone(),
                        name,
                        GraphName::DefaultGraph,
                    );
                    store.insert(&quad).unwrap();
                });
                let after_mem = PEAK_ALLOC.current_usage();
                println!("Quad size: {}", (after_mem - before_mem) / (4 * i) as usize);
                store
            })
        });
    }

    //--------------------------------------------------------------------------

    //Query

    let store = Store::new().unwrap();

    (0..1000000).for_each(|_| {
        let lover_a =
            NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat()).unwrap();
        let lover_b: NamedNode =
            NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat()).unwrap();

        let quad = Quad::new(
            lover_a.clone(),
            loves_node.clone(),
            lover_b.clone(),
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();

        let quad = Quad::new(
            lover_b.clone(),
            loves_node.clone(),
            lover_a.clone(),
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();

        let name = Literal::new_simple_literal(Name(EN).fake::<String>());
        let quad = Quad::new(
            lover_a.clone(),
            name_node.clone(),
            name,
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();

        let name = Literal::new_simple_literal(Name(EN).fake::<String>());
        let quad = Quad::new(
            lover_b.clone(),
            name_node.clone(),
            name,
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();
    });

    let juliet =
        NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat()).unwrap();
    let romeo: NamedNode =
        NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat()).unwrap();

    let quad = Quad::new(
        romeo.clone(),
        loves_node.clone(),
        juliet.clone(),
        GraphName::DefaultGraph,
    );
    store.insert(&quad).unwrap();

    let quad = Quad::new(
        juliet.clone(),
        loves_node.clone(),
        romeo.clone(),
        GraphName::DefaultGraph,
    );
    store.insert(&quad).unwrap();

    let name = Literal::new_simple_literal("Juliet");
    let quad = Quad::new(
        juliet.clone(),
        name_node.clone(),
        name,
        GraphName::DefaultGraph,
    );
    store.insert(&quad).unwrap();

    let name = Literal::new_simple_literal("Romeo");
    let quad = Quad::new(
        romeo.clone(),
        name_node.clone(),
        name,
        GraphName::DefaultGraph,
    );
    store.insert(&quad).unwrap();

    (0..1000).for_each(|_| {
        let lover_a =
            NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat()).unwrap();
        let lover_b: NamedNode =
            NamedNode::new(["urn:id:", &ufoid().encode_hex_upper::<String>()].concat()).unwrap();

        let quad = Quad::new(
            lover_a.clone(),
            loves_node.clone(),
            lover_b.clone(),
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();

        let quad = Quad::new(
            lover_b.clone(),
            loves_node.clone(),
            lover_a.clone(),
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();

        let name = Literal::new_simple_literal("Wameo");
        let quad = Quad::new(
            lover_a.clone(),
            name_node.clone(),
            name,
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();

        let name = Literal::new_simple_literal(Name(EN).fake::<String>());
        let quad = Quad::new(
            lover_b.clone(),
            name_node.clone(),
            name,
            GraphName::DefaultGraph,
        );
        store.insert(&quad).unwrap();
    });

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("sparql", 1), |b| {
        b.iter(|| {
            if let QueryResults::Solutions(solutions) =  store.query(
                "SELECT ?romeo ?juliet ?name WHERE { ?romeo <urn:id:7D4F339CC4AE0BBA2765F34BE1D108EF> \"Romeo\". ?romeo <urn:id:39E2D06DBCD9CB96DE5BC46F362CFF31> ?juliet. ?juliet <urn:id:7D4F339CC4AE0BBA2765F34BE1D108EF> ?name. }").unwrap() {
                solutions.count()
            } else {
                panic!()
            }
        })
    });

    // SPARQL query

    group.throughput(Throughput::Elements(1000));
    group.bench_function(BenchmarkId::new("sparql", 1000), |b| {
        b.iter(|| {
            if let QueryResults::Solutions(solutions) =  store.query(
                "SELECT ?romeo ?juliet ?name WHERE { ?romeo <urn:id:7D4F339CC4AE0BBA2765F34BE1D108EF> \"Wameo\". ?romeo <urn:id:39E2D06DBCD9CB96DE5BC46F362CFF31> ?juliet. ?juliet <urn:id:7D4F339CC4AE0BBA2765F34BE1D108EF> ?name. }").unwrap() {
                solutions.count()
         } else {
            panic!()
         }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    std_benchmark,
    im_benchmark,
    patch_benchmark,
    tribleset_benchmark,
    archive_benchmark,
    entities_benchmark,
    query_benchmark,
    attribute_benchmark,
    hashtribleset_benchmark,
    oxigraph_benchmark
);
criterion_main!(benches);
