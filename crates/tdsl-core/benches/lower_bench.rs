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

fn bench_lower_small(c: &mut Criterion) {
    let input = generate_tdsl(10);
    let file = tdsl_parser::parse(&input).expect("parse failed");
    c.bench_function("lower_10_spans", |b| {
        b.iter(|| tdsl_core::lower::lower_static(black_box(&file)))
    });
}

fn bench_lower_medium(c: &mut Criterion) {
    let input = generate_tdsl(100);
    let file = tdsl_parser::parse(&input).expect("parse failed");
    c.bench_function("lower_100_spans", |b| {
        b.iter(|| tdsl_core::lower::lower_static(black_box(&file)))
    });
}

fn bench_lower_large(c: &mut Criterion) {
    let input = generate_tdsl(1000);
    let file = tdsl_parser::parse(&input).expect("parse failed");
    c.bench_function("lower_1000_spans", |b| {
        b.iter(|| tdsl_core::lower::lower_static(black_box(&file)))
    });
}

fn bench_pipeline_small(c: &mut Criterion) {
    let input = generate_tdsl(10);
    c.bench_function("pipeline_parse_lower_10_spans", |b| {
        b.iter(|| {
            let file = tdsl_parser::parse(black_box(&input)).expect("parse failed");
            tdsl_core::lower::lower_static(&file)
        })
    });
}

fn bench_pipeline_large(c: &mut Criterion) {
    let input = generate_tdsl(1000);
    c.bench_function("pipeline_parse_lower_1000_spans", |b| {
        b.iter(|| {
            let file = tdsl_parser::parse(black_box(&input)).expect("parse failed");
            tdsl_core::lower::lower_static(&file)
        })
    });
}

criterion_group!(
    benches,
    bench_lower_small,
    bench_lower_medium,
    bench_lower_large,
    bench_pipeline_small,
    bench_pipeline_large
);
criterion_main!(benches);
