

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{thread_rng, Rng};
use tribles::trible::*;
use tribles::FUCID::*;
use std::iter::FromIterator;

use tribles::pact::PACT;

use im::HashSet;


fn random_tribles(length: usize) -> Vec<Trible> {
    let mut rng = thread_rng();
    let mut vec = Vec::new();

    let mut e = FUCID::new();
    let mut a = FUCID::new();
    let mut v = FUCID::new();
    for _i in 0..length {
        if rng.gen_bool(0.1) {
            e = FUCID::new();
        }
        if rng.gen_bool(0.1) {
            a = FUCID::new();
        }
        v = FUCID::new();

        vec.push(Trible::new(&e, &a, &v))
    }
    return vec;
}

fn criterion_benchmark(c: &mut Criterion) {
    let samples_10 = random_tribles(10);
    let samples_100 = random_tribles(100);
    let samples_1000 = random_tribles(1000);
    let samples_10000 = random_tribles(10000);
    let samples_100000 = random_tribles(100000);
    let samples_1000000 = random_tribles(1000000);

    c.bench_function("im insert 10", |b| {
        b.iter(|| HashSet::<Trible>::from_iter(black_box(&samples_10).iter().copied()))
    });
    c.bench_function("im insert 100", |b| {
        b.iter(|| HashSet::<Trible>::from_iter(black_box(&samples_100).iter().copied()))
    });
    c.bench_function("im insert 1000", |b| {
        b.iter(|| HashSet::<Trible>::from_iter(black_box(&samples_1000).iter().copied()))
    });
    c.bench_function("im insert 10000", |b| {
        b.iter(|| HashSet::<Trible>::from_iter(black_box(&samples_10000).iter().copied()))
    });
    c.bench_function("im insert 100000", |b| {
        b.iter(|| HashSet::<Trible>::from_iter(black_box(&samples_100000).iter().copied()))
    });
    c.bench_function("im insert 1000000", |b| {
        b.iter(|| HashSet::<Trible>::from_iter(black_box(&samples_1000000).iter().copied()))
    });

    c.bench_function("PACT insert 10", |b| {
        b.iter(|| {
            let mut pact = PACT::<64, ()>::new();
            for t in black_box(&samples_10) {
                pact.put(t.data, ());
            }
        })});
    c.bench_function("PACT insert 100", |b| {
        b.iter(|| {
            let mut pact = PACT::<64, ()>::new();
            for t in black_box(&samples_100) {
                pact.put(t.data, ());
            }
        })});
    c.bench_function("PACT insert 1000", |b| {
        b.iter(|| {
            let mut pact = PACT::<64, ()>::new();
            for t in black_box(&samples_1000) {
                pact.put(t.data, ());
            }
        })});
    c.bench_function("PACT insert 10000", |b| {
        b.iter(|| {
            let mut pact = PACT::<64, ()>::new();
            for t in black_box(&samples_10000) {
                pact.put(t.data, ());
            }
        })});
    c.bench_function("PACT insert 100000", |b| {
        b.iter(|| {
            let mut pact = PACT::<64, ()>::new();
            for t in black_box(&samples_100000) {
                pact.put(t.data, ());
            }
        })});
    c.bench_function("PACT insert 1000000", |b| {
        b.iter(|| {
            let mut pact = PACT::<64, ()>::new();
            for t in black_box(&samples_1000000) {
                pact.put(t.data, ());
            }
        })});
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);