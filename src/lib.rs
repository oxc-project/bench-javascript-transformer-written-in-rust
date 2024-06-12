pub mod oxc {
    use std::path::Path;

    use oxc::{
        allocator::Allocator,
        codegen::{Codegen, CodegenOptions},
        parser::Parser,
        span::SourceType,
        transformer::{ReactOptions, TransformOptions, Transformer, TypeScriptOptions},
    };

    pub fn transform(path: &Path, source_text: &str) -> Allocator {
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
                source_text,
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

pub mod swc {
    use std::path::Path;

    use std::sync::Arc;
    use swc::{Compiler, PrintArgs, SwcComments};
    use swc_common::{chain, source_map::SourceMap, sync::Lrc, Mark, GLOBALS};
    use swc_ecma_ast::Program;
    use swc_ecma_parser::{EsConfig, Parser, StringInput, Syntax, TsConfig};
    use swc_ecma_transforms_react::{react, Options};
    use swc_ecma_transforms_typescript::strip;
    use swc_ecma_visit::FoldWith;

    pub fn transform(path: &Path, source_text: &str) -> Program {
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
            let input = StringInput::new(source_text, Default::default(), Default::default());
            let program = Parser::new(syntax, input, Some(&comments))
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
