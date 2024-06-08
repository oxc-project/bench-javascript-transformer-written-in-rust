use std::path::Path;

use criterion::{measurement::WallTime, *};
use rayon::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

trait TheBencher {
    type RunOutput;

    const ID: &'static str;

    fn run(path: &Path, source: &str) -> Self::RunOutput;

    fn bench(g: &mut BenchmarkGroup<'_, WallTime>, path: &Path, source: &str) {
        let cpus = num_cpus::get_physical();
        let id = BenchmarkId::new(Self::ID, "single-thread");
        g.bench_with_input(id, &source, |b, source| b.iter(|| Self::run(path, source)));

        let id = BenchmarkId::new(Self::ID, "no-drop");
        g.bench_with_input(id, &source, |b, source| {
            b.iter_with_large_drop(|| Self::run(path, source))
        });

        let id = BenchmarkId::new(Self::ID, "parallel");
        g.bench_with_input(id, &source, |b, source| {
            b.iter(|| {
                (0..cpus).into_par_iter().for_each(|_| {
                    Self::run(path, source);
                });
            })
        });
    }
}

struct OxcBencher;

impl TheBencher for OxcBencher {
    type RunOutput = oxc::allocator::Allocator;

    const ID: &'static str = "oxc";

    fn run(path: &Path, source_text: &str) -> Self::RunOutput {
        use oxc::{
            allocator::Allocator,
            codegen::{Codegen, CodegenOptions},
            parser::Parser,
            span::SourceType,
            transformer::{ReactOptions, TransformOptions, Transformer, TypeScriptOptions},
        };

        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap();
        {
            let ret = Parser::new(&allocator, source_text, source_type).parse();
            let trivias = ret.trivias;
            let mut program = ret.program;
            let transform_options = TransformOptions {
                typescript: TypeScriptOptions::default(),
                react: ReactOptions::default(),
                ..TransformOptions::default()
            };
            Transformer::new(
                &allocator,
                Path::new(""),
                source_type,
                &source_text,
                &trivias,
                transform_options,
            )
            .build(&mut program)
            .unwrap();
            let _transformed_text =
                Codegen::<false>::new("", source_text, CodegenOptions::default(), None)
                    .build(&program);
        }
        allocator
    }
}

struct SwcBencher;

impl TheBencher for SwcBencher {
    type RunOutput = swc_ecma_ast::Program;

    const ID: &'static str = "swc";

    fn run(path: &Path, source: &str) -> Self::RunOutput {
        use std::sync::Arc;
        use swc::{Compiler, PrintArgs, SwcComments};
        use swc_common::{chain, source_map::SourceMap, sync::Lrc, Mark, GLOBALS};
        use swc_ecma_parser::{EsConfig, Parser, StringInput, Syntax, TsConfig};
        use swc_ecma_transforms_react::{react, Options};
        use swc_ecma_transforms_typescript::strip;
        use swc_ecma_visit::FoldWith;

        let cm = Lrc::new(SourceMap::new(swc_common::FilePathMapping::empty()));
        let compiler = Compiler::new(Arc::clone(&cm));
        let comments = SwcComments::default();
        let syntax = match path.extension().unwrap().to_str().unwrap() {
            "js" => Syntax::Es(EsConfig::default()),
            "tsx" => Syntax::Typescript(TsConfig {
                tsx: true,
                ..TsConfig::default()
            }),
            _ => panic!("need to define syntax for swc"),
        };

        GLOBALS.set(&Default::default(), || {
            let program = Parser::new(
                syntax,
                StringInput::new(source, Default::default(), Default::default()),
                Some(&comments),
            )
            .parse_program()
            .unwrap();

            let top_level_mark = Mark::new();
            let unresolved_mark = Mark::new();
            let mut ast_pass = chain!(
                strip(top_level_mark),
                react(
                    Arc::clone(&cm),
                    Some(comments),
                    Options::default(),
                    top_level_mark,
                    unresolved_mark
                ),
            );
            let program = program.fold_with(&mut ast_pass);

            let _ret = compiler
                .print(&program, PrintArgs::default())
                .expect("print failed");

            program
        })
    }
}

fn transformer_benchmark(c: &mut Criterion) {
    let filenames = ["typescript.js", "cal.com.tsx"];
    for filename in filenames {
        let path = Path::new("files").join(filename);
        let source = std::fs::read_to_string(&path).unwrap();
        let mut g = c.benchmark_group(filename);
        OxcBencher::bench(&mut g, &path, &source);
        SwcBencher::bench(&mut g, &path, &source);
        g.finish();
    }
}

criterion_group!(transformer, transformer_benchmark);
criterion_main!(transformer);
