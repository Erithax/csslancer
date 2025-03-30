use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn bench_it(c: &mut Criterion) {
    c.bench_function("bench it", |b| b.iter(|| println!("Bench it!")));
}

criterion_group!(benches, bench_it);
criterion_main!(benches);