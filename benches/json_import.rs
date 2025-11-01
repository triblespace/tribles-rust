use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::path::PathBuf;
use triblespace::blob::ToBlob;
use triblespace::id::ExclusiveId;
use triblespace::import::json::{DeterministicJsonImporter, EncodeError, JsonImporter};
use triblespace::prelude::blobschemas::LongString;
use triblespace::prelude::valueschemas::{self, Boolean, Handle, F256};
use triblespace::prelude::*;

struct Fixture {
    name: &'static str,
    payload: String,
}

fn load_fixtures() -> Vec<Fixture> {
    const FIXTURES: [(&str, &str); 3] = [
        ("canada", "canada.json"),
        ("citm_catalog", "citm_catalog.json"),
        ("twitter", "twitter.json"),
    ];

    FIXTURES
        .into_iter()
        .map(|(name, file)| {
            let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "benches", "data", "json", file]
                .into_iter()
                .collect();
            let payload = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to load {file} for {name} fixture: {err}"));
            Fixture { name, payload }
        })
        .collect()
}

fn make_importer() -> JsonImporter<
    'static,
    Handle<valueschemas::hash::Blake3, LongString>,
    F256,
    Boolean,
    impl FnMut(&str) -> Result<Value<Handle<valueschemas::hash::Blake3, LongString>>, EncodeError>,
    impl FnMut(&serde_json::Number) -> Result<Value<F256>, EncodeError>,
    impl FnMut(bool) -> Result<Value<Boolean>, EncodeError>,
    fn() -> ExclusiveId,
> {
    JsonImporter::new(
        |text: &str| {
            Ok(text
                .to_blob::<LongString>()
                .get_handle::<valueschemas::hash::Blake3>())
        },
        |number: &serde_json::Number| {
            let primitive = if let Some(n) = number.as_i64() {
                n as f64
            } else if let Some(n) = number.as_u64() {
                n as f64
            } else {
                number
                    .as_f64()
                    .ok_or_else(|| EncodeError::message("non-finite JSON number"))?
            };
            let converted = f256::f256::from_f64(primitive).ok_or_else(|| {
                EncodeError::message(format!("failed to represent {primitive} as f256"))
            })?;
            Ok(converted.to_value())
        },
        |flag: bool| Ok(Boolean::value_from(flag)),
    )
}

fn make_deterministic_importer() -> DeterministicJsonImporter<
    'static,
    Handle<valueschemas::hash::Blake3, LongString>,
    F256,
    Boolean,
    impl FnMut(&str) -> Result<Value<Handle<valueschemas::hash::Blake3, LongString>>, EncodeError>,
    impl FnMut(&serde_json::Number) -> Result<Value<F256>, EncodeError>,
    impl FnMut(bool) -> Result<Value<Boolean>, EncodeError>,
> {
    DeterministicJsonImporter::new(
        |text: &str| {
            Ok(text
                .to_blob::<LongString>()
                .get_handle::<valueschemas::hash::Blake3>())
        },
        |number: &serde_json::Number| {
            let primitive = if let Some(n) = number.as_i64() {
                n as f64
            } else if let Some(n) = number.as_u64() {
                n as f64
            } else {
                number
                    .as_f64()
                    .ok_or_else(|| EncodeError::message("non-finite JSON number"))?
            };
            let converted = f256::f256::from_f64(primitive).ok_or_else(|| {
                EncodeError::message(format!("failed to represent {primitive} as f256"))
            })?;
            Ok(converted.to_value())
        },
        |flag: bool| Ok(Boolean::value_from(flag)),
    )
}

fn json_import_benchmark(c: &mut Criterion) {
    let fixtures = load_fixtures();
    let mut group = c.benchmark_group("json_import");

    for fixture in fixtures {
        group.throughput(Throughput::Bytes(fixture.payload.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("nondeterministic", fixture.name),
            &fixture,
            |b, fixture| {
                let payload = fixture.payload.as_str();
                b.iter(|| {
                    let mut importer = make_importer();
                    importer.import_str(payload).expect("import JSON");
                    criterion::black_box(importer.data().len());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deterministic", fixture.name),
            &fixture,
            |b, fixture| {
                let payload = fixture.payload.as_str();
                b.iter(|| {
                    let mut importer = make_deterministic_importer();
                    importer.import_str(payload).expect("import JSON");
                    criterion::black_box(importer.data().len());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, json_import_benchmark);
criterion_main!(benches);
