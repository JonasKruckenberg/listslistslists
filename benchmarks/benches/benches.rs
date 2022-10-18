use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ghost_cell::GhostToken;
use std::collections::VecDeque;

#[derive(Default)]
struct Big([usize; 32]);

fn push_back_first_big(n: usize) {
    let mut list = first::LinkedList::new();

    for _ in 0..n {
        list.push_back(Big::default());
    }
}

fn push_back_second_big(n: usize) {
    GhostToken::new(|ref mut token| {
        let mut list = second::LinkedList::new();

        for _ in 0..n {
            list.push_back(Big::default(), token);
        }

        list.clear(token)
    });
}

fn push_back_third_big(n: usize) {
    let mut list = third::LinkedList::with_capacity(n);

    for _ in 0..n {
        list.push_back(Big::default());
    }
}

fn push_back_std_big(n: usize) {
    let mut list = std::collections::LinkedList::new();

    for _ in 0..n {
        list.push_back(Big::default());
    }
}

fn push_back_vecdeque_big(n: usize) {
    let mut list = VecDeque::with_capacity(n);

    for _ in 0..n {
        list.push_back(Big::default());
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("push_back_big");
    for i in [100, 300, 500, 700].iter() {
        group
            .bench_with_input(BenchmarkId::new("first", i), i, |b, i| {
                b.iter(|| push_back_first_big(*i))
            })
            .throughput(Throughput::Elements(*i as u64));

        group
            .bench_with_input(BenchmarkId::new("second", i), i, |b, i| {
                b.iter(|| push_back_second_big(*i))
            })
            .throughput(Throughput::Elements(*i as u64));

        group
            .bench_with_input(BenchmarkId::new("third", i), i, |b, i| {
                b.iter(|| push_back_third_big(*i))
            })
            .throughput(Throughput::Elements(*i as u64));

        group
            .bench_with_input(BenchmarkId::new("std", i), i, |b, i| {
                b.iter(|| push_back_std_big(*i))
            })
            .throughput(Throughput::Elements(*i as u64));

        group
            .bench_with_input(BenchmarkId::new("vecdeque", i), i, |b, i| {
                b.iter(|| push_back_vecdeque_big(*i))
            })
            .throughput(Throughput::Elements(*i as u64));
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
