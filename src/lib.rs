pub mod oxc {
    use std::path::Path;

    use oxc::{
        allocator::Allocator,
        codegen::Codegen,
        parser::Parser,
        semantic::SemanticBuilder,
        span::SourceType,
        transformer::{TransformOptions, Transformer},
    };

    pub fn transform_options() -> TransformOptions {
        TransformOptions::from_target("es2015").unwrap()
    }

    pub fn transform(
        path: &Path,
        source_text: &str,
        options: &TransformOptions,
    ) -> (Allocator, String) {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap();
        let ret = Parser::new(&allocator, source_text, source_type).parse();
        let mut program = ret.program;
        let scoping = SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping();
        let ret =
            Transformer::new(&allocator, path, options).build_with_scoping(scoping, &mut program);
        assert!(ret.errors.is_empty());
        let printed = Codegen::new().build(&program).code;

        (allocator, printed)
    }
}

pub mod swc {
    use std::{path::Path, sync::Arc};

    use swc::{try_with_handler, Compiler, PrintArgs, SwcComments};
    use swc_common::{errors::HANDLER, source_map::SourceMap, sync::Lrc, Mark, GLOBALS};
    use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax};
    use swc_ecma_transforms::{
        compat::{es2016, es2017, es2018, es2019, es2020, es2021, es2022},
        helpers::{Helpers, HELPERS},
        resolver,
    };
    use swc_ecma_transforms_react::{react, Options, Runtime};
    use swc_ecma_transforms_typescript::strip;
    use swc_ecma_visit::VisitMutWith;

    pub fn transform(path: &Path, source_text: &str) -> String {
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
            try_with_handler(cm.clone(), Default::default(), |handler| {
                HELPERS.set(&Helpers::new(true), || {
                    HANDLER.set(handler, || {
                        let input =
                            StringInput::new(source_text, Default::default(), Default::default());
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

                        let mut ast_pass = (
                            strip(unresolved_mark, top_level_mark),
                            react(
                                Arc::clone(&cm),
                                Some(comments),
                                Options {
                                    runtime: Some(Runtime::Automatic),
                                    ..Options::default()
                                },
                                top_level_mark,
                                unresolved_mark,
                            ),
                            es2022(Default::default(), unresolved_mark),
                            es2021(),
                            es2020(Default::default(), unresolved_mark),
                            es2019(),
                            es2018(Default::default()),
                            es2017(Default::default(), unresolved_mark),
                            es2016(),
                        );
                        let program = program.apply(&mut ast_pass);

                        let printed = compiler
                            .print(&program, PrintArgs::default())
                            .expect("print failed");

                        Ok(printed.code)
                    })
                })
            })
            .unwrap()
        })
    }
}
