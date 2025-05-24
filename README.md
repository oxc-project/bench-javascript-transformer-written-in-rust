# Transformer Benchmark for Oxc and Swc

A transformer is also known as a transpiler, similar to Babel transform.

The purpose of this benchmark is for people who wants to evaluate and compare the performance characteristics of the two transformer.

The benchmark measures the whole parse -> transform -> codegen pipeline as a real-word scenario.

The numbers indicate that Oxc is at least 3 times faster than Swc.

## Results

### Codspeed

[![CodSpeed Badge][codspeed-badge]][codspeed-url]

[codspeed-badge]: https://img.shields.io/endpoint?url=https://codspeed.io/badge.json
[codspeed-url]: https://codspeed.io/oxc-project/bench-javascript-transformer-written-in-rust/benchmarks

Codspeed measures performance by cpu instructions.

### Target lowering to es2015

### cal.com.tsx
|               | oxc               | swc               |
| ------------- | ----------------- | ----------------- |
| no-drop       | `11.3 ms` (1.00x) | `38.1 ms` (3.36x) |
| parallel      | `20.6 ms` (1.00x) | `83.1 ms` (4.03x) |
| single-thread | `11.3 ms` (1.00x) | `38.1 ms` (3.37x) |
### typescript.js
|               | oxc                | swc                |
| ------------- | ------------------ | ------------------ |
| no-drop       | `80.6 ms` (1.00x)  | `337.9 ms` (4.19x) |
| parallel      | `130.3 ms` (1.00x) | `725.2 ms` (5.57x) |
| single-thread | `80.3 ms` (1.00x)  | `338.0 ms` (4.21x) |

### Run benchmark locally

Run the following command on your machine for replication.

```bash
cargo bench
```

Generate the table

```bash
pnpm i
pnpm run table
```

## Maximum Resident Set Size

```
./memory.sh

./files/cal.com.tsx
oxc 26.3 mb
swc 30.9 mb

./files/typescript.js
oxc 137.8 mb
swc 148.5 mb
```

## Setup

* Uses `mimalloc` as the global allocator
* Uses the following release profile

```toml
[profile.release]
opt-level     = 3
lto           = "fat"
codegen-units = 1
strip         = "symbols"
debug         = false
panic         = "abort"
```

### single-thread

This is the standard benchmark run in a single thread.

```rust
group.bench_with_input(id, &source, |b, source| {
    b.iter(|| Self::run(source))
});
```

### no-drop

This uses the [`iter_with_large_drop`](https://docs.rs/criterion/0.5.1/criterion/struct.Bencher.html#method.iter_with_large_drop) function, which does not take AST drop time into account.

AST drop time can become a bottleneck in applications such as as bundler,
where there are a few thousands of files need to be parsed.

```rust
group.bench_with_input(id, &source, |b, source| {
    b.iter_with_large_drop(|| Self::run(source))
});
```

### parallel

This benchmark uses the total number of physical cores as the total number of files to parse per bench iteration. For example it parses 6 files in parallel on my Mac i7 6 cores.

This can indicate the existence of resource contention.

```rust
let cpus = num_cpus::get_physical();
group.bench_with_input(id, &source, |b, source| {
    b.iter(|| {
        (0..cpus).into_par_iter().for_each(|_| {
            Self::run(source);
        });
    })
});
```

