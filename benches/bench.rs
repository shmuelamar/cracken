extern crate cracken;
#[macro_use]
extern crate criterion;

use std::time::Duration;

use criterion::{Benchmark, Criterion, Throughput};

use cracken::runner;
use std::path;

fn bench_5digits(c: &mut Criterion) {
    c.bench_function("5digits", |b| b.iter(|| run_bench(vec!["?d?d?d?d?d"])));
}

fn bench_4mixed(c: &mut Criterion) {
    c.bench_function("4mixed", |b| b.iter(|| run_bench(vec!["?u?l?s?d"])));
}

fn bench_8digits_tp(c: &mut Criterion) {
    let n_elements = 100_000_000;
    let item_len = 9;
    let bencher = Benchmark::new("8digits_tp", |b| {
        b.iter(|| run_bench(vec!["?d?d?d?d?d?d?d?d"]))
    })
    .throughput(Throughput::Bytes(n_elements * item_len))
    .sample_size(10)
    .warm_up_time(Duration::new(1, 0));
    c.bench("throughput", bencher);
}

fn bench_6lower_tp(c: &mut Criterion) {
    let n_elements = 308_915_776; // 26 ** 6
    let item_len = 7;
    let bencher = Benchmark::new("6lower_tp", |b| b.iter(|| run_bench(vec!["?l?l?l?l?l?l"])))
        .throughput(Throughput::Bytes(n_elements * item_len))
        .sample_size(10)
        .warm_up_time(Duration::new(1, 0));
    c.bench("throughput", bencher);
}

fn bench_wordlist_simple(c: &mut Criterion) {
    c.bench_function("wordlist-simple", |b| {
        b.iter(|| {
            let w1 = wordlist_fname("wordlist1.txt");
            run_bench(vec!["-w", w1.as_str(), "?w1?d?d?d?d"])
        })
    });
}

fn bench_wordlist_and_custom_charset(c: &mut Criterion) {
    c.bench_function("wordlist-custom-charset", |b| {
        b.iter(|| {
            let w1 = wordlist_fname("wordlist1.txt");
            let w2 = wordlist_fname("wordlist2.txt");
            run_bench(vec![
                "-w",
                w1.as_str(),
                "-w",
                w2.as_str(),
                "-c",
                "!@#",
                "?w1?d?w2?l?w1?1",
            ])
        })
    });
}

fn bench_wordlists_charset_tp(c: &mut Criterion) {
    let bytes_size = 20_576_400;
    let bencher = Benchmark::new("wordlists_charset_tp", |b| {
        b.iter(|| {
            let w1 = wordlist_fname("wordlist1.txt");
            let w2 = wordlist_fname("wordlist2.txt");
            run_bench(vec![
                "-w",
                w1.as_str(),
                "-w",
                w2.as_str(),
                "-c",
                "!@#",
                "?w1?d?w2?l?w1?1",
            ])
        })
    })
    .throughput(Throughput::Bytes(bytes_size))
    .sample_size(10)
    .warm_up_time(Duration::new(1, 0));
    c.bench("throughput", bencher);
}

fn wordlist_fname(fname: &str) -> String {
    let mut d = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.extend(vec!["test-resources", fname]);
    d.to_str().unwrap().to_owned()
}

fn run_bench(args: Vec<&str>) {
    let mut run_args = vec!["cracken", "-o", "/dev/null"];
    run_args.extend(args);
    runner::run(Some(run_args)).unwrap();
}

criterion_group!(
    benches,
    bench_5digits,
    bench_4mixed,
    bench_wordlist_simple,
    bench_wordlist_and_custom_charset
);
criterion_group!(
    benches_throughput,
    bench_8digits_tp,
    bench_6lower_tp,
    bench_wordlists_charset_tp
);
criterion_main!(benches, benches_throughput);
