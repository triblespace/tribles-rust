use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use std::collections::HashSet;
use std::convert::TryInto;
use std::iter::FromIterator;
use tribles::and;
use tribles::attribute::Attribute;
use tribles::types::syntactic::ShortString;

use tribles::namespace::knights;
use tribles::tribleset::hashtribleset::HashTribleSet;
use tribles::types::syntactic::FUCID;
use tribles::types::syntactic::UFOID;
use tribles::{query, trible::*};

use tribles::patch::{self, Entry, IdentityOrder};
use tribles::patch::{SingleSegmentation, PATCH};
use tribles::tribleset::patchtribleset::PATCHTribleSet;

use im::OrdSet;

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

use peak_alloc::PeakAlloc;
#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

fn random_tribles(length: usize) -> Vec<Trible> {
    let mut rng = thread_rng();

    let mut vec = Vec::new();

    let mut e = UFOID::new();
    let mut a = UFOID::new();

    for _i in 0..length {
        if rng.gen_bool(0.5) {
            e = UFOID::new();
        }
        if rng.gen_bool(0.5) {
            a = UFOID::new();
        }

        let v = UFOID::new();
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
            b.iter(|| HashSet::<Trible>::from_iter(black_box(&samples).iter().copied()));
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let set = HashSet::<Trible>::from_iter(black_box(&samples).iter().copied());
            b.iter(|| set.iter().collect::<Vec<_>>());
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
                    set.add(t);
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
            b.iter(|| OrdSet::<Trible>::from_iter(black_box(&samples).iter().copied()));
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let set = OrdSet::<Trible>::from_iter(black_box(&samples).iter().copied());
            b.iter(|| set.iter().collect::<Vec<_>>());
        });
    }
    //let peak_mem = PEAK_ALLOC.peak_usage_as_gb();
    //println!("The max amount that was used {}", peak_mem);
    group.finish();
}

fn patch_benchmark(c: &mut Criterion) {
    patch::init();

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
            })
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
            for t in black_box(&samples) {
                let entry: Entry<64> = Entry::new(&t.data);
                patch.insert(&entry);
            }
            b.iter(|| patch.infixes(&[0; 64], 0, 63, |x| x))
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
                black_box(patchs.iter().fold(
                    PATCH::<64, IdentityOrder, SingleSegmentation>::new(),
                    |mut a, p| {
                        a.union(p);
                        a
                    },
                ))
            });
        });
    }

    group.finish();
}

fn tribleset_benchmark(c: &mut Criterion) {
    patch::init();

    let mut group = c.benchmark_group("tribleset");

    for i in [1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("add", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter_with_large_drop(|| {
                let before_mem = PEAK_ALLOC.current_usage_as_gb();
                let mut set = PATCHTribleSet::new();
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
            b.iter_with_large_drop(|| {
                let set = PATCHTribleSet::from_iter(black_box(samples.iter().copied()));
                set
            })
        });
    }

    /*
    let total_unioned = 1000000;
    for i in [2, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(total_unioned as u64));
        group.bench_with_input(BenchmarkId::new("union", i), i, |b, &i| {
            let samples = random_tribles(total_unioned as usize);
            let sets: Vec<_> = samples
                .chunks(total_unioned / i)
                .map(|samples| {
                    let mut set = PATCHTribleSet::new();
                    for t in samples {
                        set.add(t);
                    }
                    set
                })
                .collect();
            b.iter(|| black_box(PATCHTribleSet::union(sets.iter()).len()));
        });
    }
    */

    group.finish();
}

fn entities_benchmark(c: &mut Criterion) {
    patch::init();

    let mut group = c.benchmark_group("entities");

    group.throughput(Throughput::Elements(4));
    group.bench_function(BenchmarkId::new("entities", 4), |b| {
        b.iter(|| {
            let kb = knights::entities!((lover_a, lover_b),
            [{lover_a @
                name: Name(EN).fake::<String>().try_into().unwrap(),
                loves: lover_b
            },
            {lover_b @
                name: Name(EN).fake::<String>().try_into().unwrap(),
                loves: lover_a
            }]);
            black_box(&kb);
        })
    });

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("direct", 4 * i), |b| {
            b.iter(|| {
                let before_mem = PEAK_ALLOC.current_usage();
                let mut kb: PATCHTribleSet = PATCHTribleSet::new();
                (0..i).for_each(|_| {
                    let kb = &mut kb;
                    knights::entities!((lover_a, lover_b),
                        [{lover_a @
                            name: Name(EN).fake::<String>().try_into().unwrap(),
                            loves: lover_b
                        },
                        {lover_b @
                            name: Name(EN).fake::<String>().try_into().unwrap(),
                            loves: lover_a
                        }], kb);
                });
                let after_mem = PEAK_ALLOC.current_usage();
                println!(
                    "Trible size: {}",
                    (after_mem - before_mem) / kb.len() as usize
                );
                black_box(&kb);
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union", 4 * i), |b| {
            b.iter_with_large_drop(|| {
                let kb = (0..i)
                    .map(|_| {
                        knights::entities!((lover_a, lover_b),
                        [{lover_a @
                            name: Name(EN).fake::<String>().try_into().unwrap(),
                            loves: lover_b
                        },
                        {lover_b @
                            name: Name(EN).fake::<String>().try_into().unwrap(),
                            loves: lover_a
                        }])
                    })
                    .fold(PATCHTribleSet::new(), |mut kb, set| {
                        kb.union(&set);
                        kb
                    });
                black_box(&kb);
                kb
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union/prealloc", 4 * i), |b| {
            let sets: Vec<_> = (0..i)
                .map(|_| {
                    knights::entities!((lover_a, lover_b),
                    [{lover_a @
                        name: Name(EN).fake::<String>().try_into().unwrap(),
                        loves: lover_b
                    },
                    {lover_b @
                        name: Name(EN).fake::<String>().try_into().unwrap(),
                        loves: lover_a
                    }])
                })
                .collect();
            b.iter_with_large_drop(|| {
                let mut kb = PATCHTribleSet::new();
                for set in &sets {
                    kb.union(&set);
                }
                black_box(&kb);
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
                    .map(|_| {
                        knights::entities!((lover_a, lover_b),
                        [{lover_a @
                            name: Name(EN).fake::<String>().try_into().unwrap(),
                            loves: lover_b
                        },
                        {lover_b @
                            name: Name(EN).fake::<String>().try_into().unwrap(),
                            loves: lover_a
                        }])
                    })
                    .reduce(
                        || PATCHTribleSet::new(),
                        |mut a, b| {
                            a.union(&b);
                            a
                        },
                    );
                black_box(&kb);
                kb
            })
        });
    }

    for i in [1000000] {
        group.sample_size(10);
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union/parallel/prealloc", 4 * i), |b| {
            let sets: Vec<_> = (0..i)
                .map(|_| {
                    knights::entities!((lover_a, lover_b),
                    [{lover_a @
                        name: Name(EN).fake::<String>().try_into().unwrap(),
                        loves: lover_b
                    },
                    {lover_b @
                        name: Name(EN).fake::<String>().try_into().unwrap(),
                        loves: lover_a
                    }])
                })
                .collect();
            b.iter_with_large_drop(|| {
                let kb = sets.par_iter().cloned().reduce(
                    || PATCHTribleSet::new(),
                    |mut a, b| {
                        a.union(&b);
                        a
                    },
                );
                black_box(&kb);
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
                            .map(|_| {
                                knights::entities!((lover_a, lover_b),
                                [{lover_a @
                                    name: Name(EN).fake::<String>().try_into().unwrap(),
                                    loves: lover_b
                                },
                                {lover_b @
                                    name: Name(EN).fake::<String>().try_into().unwrap(),
                                    loves: lover_a
                                }])
                            })
                            .fold(PATCHTribleSet::new(), |mut kb, set| {
                                kb.union(&set);
                                kb
                            })
                    })
                    .reduce(
                        || PATCHTribleSet::new(),
                        |mut a, b| {
                            a.union(&b);
                            a
                        },
                    );
                black_box(&kb);
                kb
            })
        });
    }

    group.finish();
}

fn query_benchmark(c: &mut Criterion) {
    patch::init();

    let mut group = c.benchmark_group("query");

    let mut kb = PATCHTribleSet::new();
    (0..1000000).for_each(|_| {
        kb.union(&knights::entities!((lover_a, lover_b),
        [{lover_a @
            name: Name(EN).fake::<String>().try_into().unwrap(),
            loves: lover_b
        },
        {lover_b @
            name: Name(EN).fake::<String>().try_into().unwrap(),
            loves: lover_a
        }]));
    });

    let mut data_kb = knights::entities!((romeo, juliet),
    [{juliet @
        name: "Juliet".try_into().unwrap(),
        loves: romeo
    },
    {romeo @
        name: "Romeo".try_into().unwrap(),
        loves: juliet
    }]);

    (0..1000).for_each(|_| {
        data_kb.union(&knights::entities!((lover_a, lover_b),
        [{lover_a @
            name: "Wameo".try_into().unwrap(),
            loves: lover_b
        },
        {lover_b @
            name: Name(EN).fake::<String>().try_into().unwrap(),
            loves: lover_a
        }]));
    });

    kb.union(&data_kb);

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("pattern", 1), |b| {
        b.iter_with_large_drop(|| {
            let r = query!(
                ctx,
                (juliet, name),
                knights::pattern!(ctx, kb, [
                {name: ("Romeo".try_into().unwrap()),
                 loves: juliet},
                {juliet @
                    name: name
                }])
            )
            .count();
            black_box(r)
        })
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function(BenchmarkId::new("pattern", 1000), |b| {
        b.iter_with_large_drop(|| {
            let r = query!(
                ctx,
                (juliet, name),
                knights::pattern!(ctx, kb, [
                {name: ("Wameo".try_into().unwrap()),
                 loves: juliet},
                {juliet @
                    name: name
                }])
            )
            .count();
            black_box(r)
        })
    });
    group.finish();
}

fn attribute_benchmark(c: &mut Criterion) {
    patch::init();

    let mut group = c.benchmark_group("attribute");

    let mut name: Attribute<UFOID, ShortString> = Attribute::new();
    let mut loves: Attribute<UFOID, UFOID> = Attribute::new();

    (0..1000000).for_each(|_| {
        let lover_a = UFOID::new();
        let lover_b = UFOID::new();
        name.add(&lover_a, &(Name(EN).fake::<String>().try_into().unwrap()));
        name.add(&lover_b, &(Name(EN).fake::<String>().try_into().unwrap()));
        loves.add(&lover_a, &lover_b);
        loves.add(&lover_b, &lover_a);
    });

    (0..1000).for_each(|_| {
        let lover_a = UFOID::new();
        let lover_b = UFOID::new();
        name.add(&lover_a, &("Wameo".try_into().unwrap()));
        name.add(&lover_b, &(Name(EN).fake::<String>().try_into().unwrap()));
        loves.add(&lover_a, &lover_b);
        loves.add(&lover_b, &lover_a);
    });

    let romeo = UFOID::new();
    let juliet = UFOID::new();
    name.add(&romeo, &("Romeo".try_into().unwrap()));
    name.add(&juliet, &("Juliet".try_into().unwrap()));
    loves.add(&romeo, &juliet);
    loves.add(&juliet, &romeo);

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("query", 1), |b| {
        b.iter_with_large_drop(|| {
            let r = query!(
                ctx,
                (juliet, romeo, romeo_name, juliet_name),
                and!(
                    romeo_name.is("Romeo".try_into().unwrap()),
                    name.has(romeo, romeo_name),
                    name.has(juliet, juliet_name),
                    loves.has(romeo, juliet)
                )
            )
            .count();
            black_box(r)
        })
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function(BenchmarkId::new("query", 1000), |b| {
        b.iter_with_large_drop(|| {
            let r = query!(
                ctx,
                (juliet, romeo, romeo_name, juliet_name),
                and!(
                    romeo_name.is("Wameo".try_into().unwrap()),
                    name.has(romeo, romeo_name),
                    name.has(juliet, juliet_name),
                    loves.has(romeo, juliet)
                )
            )
            .count();
            black_box(r)
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
    entities_benchmark,
    query_benchmark,
    attribute_benchmark,
    hashtribleset_benchmark,
);
criterion_main!(benches);
