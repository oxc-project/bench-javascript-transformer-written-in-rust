#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{env, fs, path::Path};

use bench_transformer::swc;

pub fn main() {
    let path = env::args().nth(1).unwrap();
    let path = Path::new(&path);
    let source_text = fs::read_to_string(path).unwrap();
    let _output = swc::transform(path, &source_text);
    // println!("{}", output.1);
}
