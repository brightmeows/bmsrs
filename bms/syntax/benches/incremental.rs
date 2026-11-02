//! Incremental parser benchmarks.

use std::fmt::Write as _;
use std::hint::black_box;

use bmsrs_bms_syntax::{IncrementalParser, ParseOptions, SourceFile};
use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};

/// Build an old and a single-line-edited chart.
fn chart(lines: usize) -> (String, String) {
    let mut old = String::new();
    for index in 0..lines {
        writeln!(old, "#TITLE {index}").unwrap_or_default();
    }
    let new = old.replacen("#TITLE 0", "#TITLE X", 1);
    (old, new)
}

/// Benchmark full parse against a single-line incremental update.
fn bench_incremental(c: &mut Criterion) {
    for lines in [1_000, 10_000, 20_000, 50_000] {
        let (old, new) = chart(lines);
        let mut group = c.benchmark_group("incremental");
        group.sample_size(10);

        group.bench_with_input(BenchmarkId::new("full_parse", lines), &new, |b, text| {
            b.iter(|| SourceFile::parse(black_box(text)));
        });

        group.bench_with_input(
            BenchmarkId::new("update", lines),
            &(old, new),
            |b, (old_text, new_text)| {
                b.iter_batched(
                    || IncrementalParser::new(old_text, ParseOptions::default()),
                    |mut parser| {
                        parser.update(black_box(new_text));
                        black_box(parser.parse().source_file().syntax().to_string());
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.finish();
    }
}

criterion_group!(benches, bench_incremental);
criterion_main!(benches);
