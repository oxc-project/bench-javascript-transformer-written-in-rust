pub mod oxc {
    use std::path::Path;

    use oxc::{
        allocator::Allocator,
        codegen::CodeGenerator,
        parser::Parser,
        semantic::SemanticBuilder,
        span::SourceType,
        transformer::{ReactOptions, TransformOptions, Transformer, TypeScriptOptions},
    };

    pub fn transform(path: &Path, source_text: &str) -> (Allocator, String) {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap();
        let printed = {
            let ret = Parser::new(&allocator, source_text, source_type).parse();
            let trivias = ret.trivias;
            let mut program = ret.program;
            let transform_options = TransformOptions {
                typescript: TypeScriptOptions::default(),
                react: ReactOptions::default(),
                ..TransformOptions::default()
            };
            let (symbols, scopes) = SemanticBuilder::new(source_text, source_type)
                .build(&program)
                .semantic
                .into_symbol_table_and_scope_tree();
            let ret = Transformer::new(
                &allocator,
                path,
                source_type,
                source_text,
                trivias.clone(),
                transform_options,
            )
            .build_with_symbols_and_scopes(symbols, scopes, &mut program);
            assert!(ret.errors.is_empty());
            CodeGenerator::new().build(&program).source_text
        };

        (allocator, printed)
    }
}

pub mod swc {
    use std::{path::Path, sync::Arc};

    use swc::{Compiler, PrintArgs, SwcComments};
    use swc_common::{chain, source_map::SourceMap, sync::Lrc, Mark, GLOBALS};
    use swc_ecma_ast::Program;
    use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax};
    use swc_ecma_transforms::resolver;
    use swc_ecma_transforms_react::{react, Options, Runtime};
    use swc_ecma_transforms_typescript::strip;
    use swc_ecma_visit::FoldWith;
    use swc_ecma_visit::VisitMutWith;

    pub fn transform(path: &Path, source_text: &str) -> (Program, String) {
        let cm = Lrc::new(SourceMap::new(swc_common::FilePathMapping::empty()));
        let compiler = Compiler::new(Arc::clone(&cm));
        let comments = SwcComments::default();
        let syntax = match path.extension().unwrap().to_str().unwrap() {
            "js" => Syntax::Es(EsSyntax::default()),
            "tsx" => Syntax::Typescript(TsSyntax {
                tsx: true,
                ..TsSyntax::default()
            }),
            _ => panic!("need to define syntax for swc"),
        };

        GLOBALS.set(&Default::default(), || {
            let input = StringInput::new(source_text, Default::default(), Default::default());
            let mut program = Parser::new(syntax, input, Some(&comments))
                .parse_program()
                .unwrap();

            let top_level_mark = Mark::new();
            let unresolved_mark = Mark::new();

            program.visit_mut_with(&mut resolver(
                unresolved_mark,
                top_level_mark,
                syntax.typescript(),
            ));

            let mut ast_pass = chain!(
                strip(unresolved_mark, top_level_mark),
                react(
                    Arc::clone(&cm),
                    Some(comments),
                    Options {
                        runtime: Some(Runtime::Automatic),
                        ..Options::default()
                    },
                    top_level_mark,
                    unresolved_mark
                ),
            );
            let program = program.fold_with(&mut ast_pass);

            let printed = compiler
                .print(&program, PrintArgs::default())
                .expect("print failed");

            (program, printed.code)
        })
    }
}
