use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{thread_rng, Rng};
use std::iter::FromIterator;
use tribles::fucid::FUCID;
use tribles::trible::*;
use tribles::ufoid::UFOID;

use tribles::pact;
use tribles::pact::PACT;

use im::OrdSet;

fn random_tribles(length: usize) -> Vec<Trible> {
    let mut rng = thread_rng();
    let mut vec = Vec::new();

    let mut e = UFOID::new(&mut rng);
    let mut a = UFOID::new(&mut rng);
    let mut v = UFOID::new(&mut rng);
    for _i in 0..length {
        if rng.gen_bool(0.1) {
            e = UFOID::new(&mut rng);
        }
        if rng.gen_bool(0.1) {
            a = UFOID::new(&mut rng);
        }
        v = UFOID::new(&mut rng);

        vec.push(Trible::new(&e, &a, &v))
    }
    return vec;
}

fn im_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("im");

    for i in [10, 100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("put", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| OrdSet::<Trible>::from_iter(black_box(&samples).iter().copied()));
        });
    }
    group.finish();
}

fn pact_benchmark(c: &mut Criterion) {
    pact::init();

    let mut group = c.benchmark_group("PACT");

    for i in [10, 100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*i));
        group.bench_with_input(BenchmarkId::new("current", i), i, |b, &i| {
            let samples = random_tribles(i as usize);
            b.iter(|| {
                let mut pact = PACT::<64>::new();
                for t in black_box(&samples) {
                    pact.put(t.data);
                }
            })
        });
    }
    group.finish();
}

criterion_group!(benches, im_benchmark, pact_benchmark);
criterion_main!(benches);
