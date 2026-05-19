use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn generate_tdsl(n: usize) -> String {
    let mut s = String::new();
    s.push_str(
        r#"timeline "Bench Timeline" {
    title "Bench Timeline";
    unit year;
    range 0..10000;
    calendar proleptic_gregorian;
}
lane "BenchLane" as bench { kind custom; order 1; }
"#,
    );
    for i in 0..n {
        let start = i as i64 * 10;
        let end = start + 9;
        s.push_str(&format!(
            "span bench {}..{} \"Item {}\" {{ tags [\"t\"]; }};\n",
            start, end, i
        ));
    }
    s
}

fn bench_parse_small(c: &mut Criterion) {
    let input = generate_tdsl(10);
    c.bench_function("parse_10_spans", |b| {
        b.iter(|| tdsl_parser::parse(black_box(&input)))
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    let input = generate_tdsl(100);
    c.bench_function("parse_100_spans", |b| {
        b.iter(|| tdsl_parser::parse(black_box(&input)))
    });
}

fn bench_parse_large(c: &mut Criterion) {
    let input = generate_tdsl(1000);
    c.bench_function("parse_1000_spans", |b| {
        b.iter(|| tdsl_parser::parse(black_box(&input)))
    });
}

criterion_group!(
    benches,
    bench_parse_small,
    bench_parse_medium,
    bench_parse_large
);
criterion_main!(benches);
