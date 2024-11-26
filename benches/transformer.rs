use std::path::Path;

use criterion::{measurement::WallTime, *};
use rayon::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

trait TheBencher {
    type RunOutput;
    type Options: Send + Sync;

    const ID: &'static str;

    fn run(path: &Path, source: &str, options: &Self::Options) -> Self::RunOutput;

    fn bench(
        g: &mut BenchmarkGroup<'_, WallTime>,
        path: &Path,
        source: &str,
        options: &Self::Options,
    ) {
        let cpus = num_cpus::get_physical();
        let id = BenchmarkId::new(Self::ID, "single-thread");
        g.bench_with_input(id, &source, |b, source| {
            b.iter(|| Self::run(path, source, options))
        });

        let id = BenchmarkId::new(Self::ID, "no-drop");
        g.bench_with_input(id, &source, |b, source| {
            b.iter_with_large_drop(|| Self::run(path, source, options))
        });

        let id = BenchmarkId::new(Self::ID, "parallel");
        g.bench_with_input(id, &source, |b, source| {
            b.iter(|| {
                (0..cpus).into_par_iter().for_each(|_| {
                    Self::run(path, source, options);
                });
            })
        });
    }
}

struct OxcBencher;

impl TheBencher for OxcBencher {
    type RunOutput = (oxc::allocator::Allocator, String);
    type Options = oxc::transformer::TransformOptions;

    const ID: &'static str = "oxc";

    fn run(path: &Path, source_text: &str, options: &Self::Options) -> Self::RunOutput {
        bench_transformer::oxc::transform(path, source_text, options)
    }
}

struct SwcBencher;

impl TheBencher for SwcBencher {
    type RunOutput = String;
    type Options = ();

    const ID: &'static str = "swc";

    fn run(path: &Path, source_text: &str, _option: &Self::Options) -> Self::RunOutput {
        bench_transformer::swc::transform(path, source_text)
    }
}

fn transformer_benchmark(c: &mut Criterion) {
    let filenames = ["typescript.js", "cal.com.tsx"];
    for filename in filenames {
        let path = Path::new("files").join(filename);
        let source = std::fs::read_to_string(&path).unwrap();
        let mut g = c.benchmark_group(filename);

        let options = oxc::transformer::TransformOptions::from_target("es2015").unwrap();
        OxcBencher::bench(&mut g, &path, &source, &options);

        SwcBencher::bench(&mut g, &path, &source, &());
        g.finish();
    }
}

criterion_group!(transformer, transformer_benchmark);
criterion_main!(transformer);
