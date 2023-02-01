use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{thread_rng, Rng};
use std::iter::FromIterator;
use tribles::fucid::FUCID;
use tribles::trible::*;
use tribles::ufoid::UFOID;

use tribles::pact;
use tribles::pact::PACT;
use tribles::tribleset::TribleSet;

use im::OrdSet;

use peak_alloc::PeakAlloc;
#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

fn random_tribles(length: usize) -> Vec<Trible> {
    let mut rng = thread_rng();

    let mut vec = Vec::new();

    let mut e = UFOID::new();
    let mut a = UFOID::new();
    let mut v = UFOID::new();
    for _i in 0..length {
        if rng.gen_bool(0.5) {
            e = UFOID::new();
        }
        if rng.gen_bool(0.5) {
            a = UFOID::new();
        }
        v = UFOID::new();

        vec.push(Trible::new(&e, &a, &v))
    }
    return vec;
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
                let mut pact = PACT::<64>::new();
                for t in black_box(&samples) {
                    pact.put(t.data);
                }
            })
        });
        group.bench_with_input(BenchmarkId::new("iter", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let mut pact = PACT::<64>::new();
            for t in black_box(&samples) {
                pact.put(t.data);
            }
            b.iter(|| pact.cursor().into_iter().count())
        });
    }

    let total_unioned = 1000000;
    for i in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(total_unioned as u64));
        group.bench_with_input(BenchmarkId::new("union", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            let pacts = samples.chunks(total_unioned / i).map(|samples| {
                let mut pact = PACT::<64>::new();
                for t in samples {
                    pact.put(t.data);
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
        group.bench_with_input(BenchmarkId::new("put", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                let mut set = TribleSet::new();
                for t in black_box(&samples) {
                    set.put(t);
                }
                let peak_mem = PEAK_ALLOC.peak_usage_as_gb();
                println!("The max amount that was used {}", peak_mem);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, im_benchmark, pact_benchmark, tribleset_benchmark);
criterion_main!(benches);
