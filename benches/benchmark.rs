use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::iter::FromIterator;
use tribles::hashtribleset::HashTribleSet;
use tribles::trible::*;
use tribles::types::fucid::FUCID;
use tribles::types::ufoid::UFOID;
use triomphe::Arc;

use tribles::pact::{self, IdentityOrder};
use tribles::pact::{SingleSegmentation, PACT};
use tribles::tribleset::PACTTribleSet;

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
            b.iter(|| set.iter().count());
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
            b.iter(|| set.iter().count());
        });
    }
    //let peak_mem = PEAK_ALLOC.peak_usage_as_gb();
    //println!("The max amount that was used {}", peak_mem);
    group.finish();
}

fn pact_benchmark(c: &mut Criterion) {
    pact::init();

    let mut group = c.benchmark_group("pact");

    for i in [10, 100, 1000, 10000, 100000, 1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("put", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                let mut pact = PACT::<64, IdentityOrder, SingleSegmentation>::new();
                for t in black_box(&samples) {
                    pact.put(&Arc::new(t.data));
                }
            })
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut pact = PACT::<64, IdentityOrder, SingleSegmentation>::new();
            for t in black_box(&samples) {
                pact.put(&Arc::new(t.data));
            }
            b.iter(|| pact.infixes([0; 64], 0, 63, |x| x))
        });
    }

    let total_unioned = 1000000;
    for i in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(total_unioned as u64));
        group.bench_with_input(BenchmarkId::new("union", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let pacts = samples.chunks(total_unioned / i).map(|samples| {
                let mut pact = PACT::<64, IdentityOrder, SingleSegmentation>::new();
                for t in samples {
                    pact.put(&Arc::new(t.data));
                }
                pact
            });
            b.iter(|| PACT::union(black_box(pacts.clone())));
        });
    }

    group.finish();
}

fn tribleset_benchmark(c: &mut Criterion) {
    pact::init();

    let mut group = c.benchmark_group("tribleset");

    for i in [1000000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("add", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                let before_mem = PEAK_ALLOC.current_usage_as_gb();
                let mut set = PACTTribleSet::new();
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
                let _set = PACTTribleSet::from_iter(black_box(samples.iter().copied()));
            })
        });
    }

    let total_unioned = 1000000;
    for i in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(total_unioned as u64));
        group.bench_with_input(BenchmarkId::new("union", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let sets = samples.chunks(total_unioned / i).map(|samples| {
                let mut set = PACTTribleSet::new();
                for t in samples {
                    set.add(t);
                }
                set
            });
            b.iter(|| PACTTribleSet::union(black_box(sets.clone())));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    std_benchmark,
    im_benchmark,
    pact_benchmark,
    tribleset_benchmark,
    hashtribleset_benchmark,
);
criterion_main!(benches);
