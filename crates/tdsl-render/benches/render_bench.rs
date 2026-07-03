use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};
use tdsl_render::RenderOptions;

fn make_ir(n: usize) -> TimelineIr {
    let mut items = Vec::with_capacity(n);
    for i in 0..n {
        let start = i as i64 * 10;
        items.push(Item::Span {
            id: format!("span:bench:{}", i),
            lane: "bench".into(),
            start,
            end: start + 9,
            label: format!("Item {}", i),
            tags: vec!["t".into()],
            source: None,
            origin: None,
            start_month: None,
            start_day: None,
            start_hour: None,
            start_minute: None,
            end_month: None,
            end_day: None,
            end_hour: None,
            end_minute: None,
            end_open: false,
            source_span: None,
        });
    }
    TimelineIr {
        meta: Meta {
            title: "Bench Timeline".into(),
            unit: "year".into(),
            range: (0, n as i64 * 10),
            calendar: "proleptic_gregorian".into(),
            color_map: Default::default(),
            ..Default::default()
        },
        lanes: vec![Lane {
            id: "bench".into(),
            label: "BenchLane".into(),
            kind: "custom".into(),
            order: 1,
            group: None,
            source_span: None,
        }],
        items,
        imports: vec![],
        sources: vec![],
    }
}

fn bench_render_small(c: &mut Criterion) {
    let ir = make_ir(10);
    c.bench_function("render_html_10_spans", |b| {
        b.iter(|| tdsl_render::render_html(black_box(&ir), RenderOptions::default()))
    });
}

fn bench_render_medium(c: &mut Criterion) {
    let ir = make_ir(100);
    c.bench_function("render_html_100_spans", |b| {
        b.iter(|| tdsl_render::render_html(black_box(&ir), RenderOptions::default()))
    });
}

fn bench_render_large(c: &mut Criterion) {
    let ir = make_ir(1000);
    c.bench_function("render_html_1000_spans", |b| {
        b.iter(|| tdsl_render::render_html(black_box(&ir), RenderOptions::default()))
    });
}

criterion_group!(
    benches,
    bench_render_small,
    bench_render_medium,
    bench_render_large
);
criterion_main!(benches);
