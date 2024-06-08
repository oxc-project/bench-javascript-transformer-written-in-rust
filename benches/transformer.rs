use std::path::Path;

use criterion::{measurement::WallTime, *};
use rayon::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

trait TheBencher {
    type RunOutput;

    const ID: &'static str;

    fn run(source: &str) -> Self::RunOutput;

    fn bench(g: &mut BenchmarkGroup<'_, WallTime>, source: &str) {
        let cpus = num_cpus::get_physical();
        let id = BenchmarkId::new(Self::ID, "single-thread");
        g.bench_with_input(id, &source, |b, source| b.iter(|| Self::run(source)));

        let id = BenchmarkId::new(Self::ID, "no-drop");
        g.bench_with_input(id, &source, |b, source| {
            b.iter_with_large_drop(|| Self::run(source))
        });

        let id = BenchmarkId::new(Self::ID, "parallel");
        g.bench_with_input(id, &source, |b, source| {
            b.iter(|| {
                (0..cpus).into_par_iter().for_each(|_| {
                    Self::run(source);
                });
            })
        });
    }
}

struct OxcBencher;

impl TheBencher for OxcBencher {
    type RunOutput = oxc::allocator::Allocator;

    const ID: &'static str = "oxc";

    fn run(source_text: &str) -> Self::RunOutput {
        use oxc::{
            allocator::Allocator,
            codegen::{Codegen, CodegenOptions},
            parser::Parser,
            span::SourceType,
            transformer::{ReactOptions, TransformOptions, Transformer, TypeScriptOptions},
        };

        let allocator = Allocator::default();
        let source_type = SourceType::default();
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

    fn run(source: &str) -> Self::RunOutput {
        use std::sync::Arc;
        use swc::{Compiler, PrintArgs, SwcComments};
        use swc_common::{chain, source_map::SourceMap, sync::Lrc, Mark, GLOBALS};
        use swc_ecma_parser::{Parser, StringInput, Syntax};
        use swc_ecma_transforms_react::{react, Options};
        use swc_ecma_transforms_typescript::strip;
        use swc_ecma_visit::FoldWith;

        let cm = Lrc::new(SourceMap::new(swc_common::FilePathMapping::empty()));
        let compiler = Compiler::new(Arc::clone(&cm));
        let comments = SwcComments::default();

        GLOBALS.set(&Default::default(), || {
            let program = Parser::new(
                Syntax::Es(Default::default()),
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
    let filename = "typescript.js";
    let source = std::fs::read_to_string(filename).unwrap();

    let mut g = c.benchmark_group(filename);
    OxcBencher::bench(&mut g, &source);
    SwcBencher::bench(&mut g, &source);
    g.finish();
}

criterion_group!(transformer, transformer_benchmark);
criterion_main!(transformer);
