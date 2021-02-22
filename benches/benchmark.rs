use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{thread_rng, Rng};
use tribles::trible::*;
use tribles::tribledb::imtribledb::ImTribleDB;
use tribles::tribledb::imtribledb2::ImTribleDB2;
use tribles::tribledb::imtribledb3::ImTribleDB3;
use tribles::tribledb::TribleDB;

fn random_tribles(length: usize) -> Vec<Trible> {
    let mut rng = thread_rng();
    let mut vec = Vec::new();
    let mut e = rng.gen();
    let mut a = rng.gen();
    let mut v1 = rng.gen();
    let mut v2 = rng.gen();
    for _i in 0..length {
        if rng.gen_bool(0.1) {
            e = rng.gen();
        }
        if rng.gen_bool(0.1) {
            a = rng.gen();
        }
        if rng.gen_bool(0.8) {
            v1 = rng.gen();
        }
        if rng.gen_bool(0.8) {
            v2 = rng.gen();
        }
        vec.push(Trible {
            e: E(e),
            a: A(a),
            v1: V1(v1),
            v2: V2(v2),
        })
    }
    return vec;
}

fn criterion_benchmark(c: &mut Criterion) {
    let samples_10 = random_tribles(10);
    let samples_100 = random_tribles(100);
    let samples_1000 = random_tribles(1000);
    let samples_10000 = random_tribles(10000);
    let tribledb: ImTribleDB3 = Default::default();

    c.bench_function("insert 10", |b| {
        b.iter(|| (black_box(&tribledb).with(samples_10.iter())))
    });
    c.bench_function("insert 100", |b| {
        b.iter(|| (black_box(&tribledb).with(samples_100.iter())))
    });
    c.bench_function("insert 1000", |b| {
        b.iter(|| (black_box(&tribledb).with(samples_1000.iter())))
    });
    c.bench_function("insert 10000", |b| {
        b.iter(|| (black_box(&tribledb).with(samples_10000.iter())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
