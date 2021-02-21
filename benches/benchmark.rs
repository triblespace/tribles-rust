use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{thread_rng, Rng};
use tribles::tribledb::imtribledb::ImTribleDB;
use tribles::tribledb::TribleDB;
use tribles::trible::*;

fn random_tribles(length: usize) -> Vec<Trible> {
    let mut rng = thread_rng();
    let mut vec = Vec::new();
    for i in 0..length {
        vec.push(Trible {e: E(rng.gen()),
                         a: A(rng.gen()),
                         v1: V1(rng.gen()),
                         v2: V2(rng.gen())})
    }
    return vec;
}

fn criterion_benchmark(c: &mut Criterion) {

    let samples_10 = random_tribles(10);
    let tribledb: ImTribleDB = Default::default();

    c.bench_function("insert 10", |b| b.iter(|| (black_box(tribledb.with(&samples_10)))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
