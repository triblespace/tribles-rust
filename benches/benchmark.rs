use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::convert::TryInto;
use std::iter::FromIterator;
use tribles::namespace::knights;
use tribles::tribleset::hashtribleset::HashTribleSet;
use tribles::types::syntactic::FUCID;
use tribles::types::syntactic::UFOID;
use tribles::{query, trible::*};

use tribles::patch::{self, Entry, IdentityOrder};
use tribles::patch::{SingleSegmentation, PATCH};
use tribles::tribleset::patchtribleset::PATCHTribleSet;

use im::OrdSet;

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
            b.iter(|| {
                let before_mem = PEAK_ALLOC.current_usage_as_gb();
                let mut set = HashTribleSet::new();
                for t in black_box(&samples) {
                    set.add(t);
                }
                let after_mem = PEAK_ALLOC.current_usage_as_gb();
                println!("HashTribleset size: {}", after_mem - before_mem);
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
                    patch.put(&entry);
                }
            })
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut patch = PATCH::<64, IdentityOrder, SingleSegmentation>::new();
            for t in black_box(&samples) {
                let entry: Entry<64> = Entry::new(&t.data);
                patch.put(&entry);
            }
            b.iter(|| patch.infixes([0; 64], 0, 63, |x| x))
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
                        patch.put(&entry);
                    }
                    patch
                })
                .collect();
            b.iter(|| black_box(PATCH::union(patchs.iter())));
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
            b.iter(|| {
                let before_mem = PEAK_ALLOC.current_usage_as_gb();
                let mut set = PATCHTribleSet::new();
                for t in black_box(&samples) {
                    set.add(t);
                }
                let after_mem = PEAK_ALLOC.current_usage_as_gb();
                println!("Tribleset size: {}", after_mem - before_mem);
            })
        });
    }

    for i in [1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("from_iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                let _set = PATCHTribleSet::from_iter(black_box(samples.iter().copied()));
            })
        });
    }

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

    group.finish();
}

use fake::faker::name::raw::*;
use fake::locales::*;
use fake::{Dummy, Fake, Faker};

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

    for i in [1, 10, 100, 1000, 10000, 100000, 1000000] {
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("direct", 4 * i), |b| {
            b.iter(|| {
                let before_mem = PEAK_ALLOC.current_usage();
                let mut kb = PATCHTribleSet::new();
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
                println!("Trible size: {}", (after_mem - before_mem) / kb.len() as usize);
                black_box(&kb);
            })
        });
    }

    for i in [1, 10, 100, 1000, 10000, 100000, 1000000] {
        group.throughput(Throughput::Elements(4 * i));
        group.bench_function(BenchmarkId::new("union", 4 * i), |b| {
            b.iter(|| {
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
                    .fold(PATCHTribleSet::new(), |u, n| {
                        PATCHTribleSet::union([u, n].iter())
                    });
                black_box(&kb);
            })
        });
    }

    group.finish();
}

fn query_benchmark(c: &mut Criterion) {
    patch::init();

    let mut group = c.benchmark_group("query");

    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("pattern", 1), |b| {
        let juliet = knights::Id::new();
        let background_kbs: Vec<_> = (0..1000000)
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

        let background_kb = PATCHTribleSet::union(background_kbs.iter());

        let data_kb = knights::entities!((romeo),
        [{juliet @
            name: "Juliet".try_into().unwrap(),
            loves: romeo
        },
        {romeo @
            name: "Romeo".try_into().unwrap(),
            loves: juliet
        }]);

        let kb = PATCHTribleSet::union([background_kb, data_kb].iter());

        b.iter(|| {
            let r: Vec<_> = query!(
                ctx,
                (juliet, name),
                knights::pattern!(ctx, kb, [
                {name: ("Romeo".try_into().unwrap()),
                 loves: juliet},
                {juliet @
                    name: name
                }])
            )
            .collect();
            black_box(&r);
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
    hashtribleset_benchmark,
);
criterion_main!(benches);
