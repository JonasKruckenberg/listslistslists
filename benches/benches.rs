use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ghost_cell::GhostToken;

#[derive(Default)]
struct Small(usize);

#[derive(Default)]
struct Big([usize; 32]);

fn push_back_crate_small(n: usize) {
    GhostToken::new(|ref mut token| {
        let mut list = linked_list::LinkedList::new();

        for _ in 0..n {
            list.push_back(Small::default(), token);
        }

        list.clear(token);
    });
}

fn push_back_std_small(n: usize) {
    let mut list = std::collections::LinkedList::new();

    for _ in 0..n {
        list.push_back(Small::default());
    }

    list.clear();
}

fn push_back_crate_big(n: usize) {
    GhostToken::new(|ref mut token| {
        let mut list = linked_list::LinkedList::new();

        for _ in 0..n {
            list.push_back(Big::default(), token);
        }

        list.clear(token);
    });
}

fn push_back_std_big(n: usize) {
    let mut list = std::collections::LinkedList::new();

    for _ in 0..n {
        list.push_back(Big::default());
    }

    list.clear();
}

fn bench_fibs(c: &mut Criterion) {
    let mut group = c.benchmark_group("push_back_small");
    for i in [20, 40, 60, 80].iter() {
        group
            .bench_with_input(BenchmarkId::new("crate", i), i, |b, i| {
                b.iter(|| push_back_crate_small(*i))
            })
            .throughput(Throughput::Elements(*i as u64));
        group
            .bench_with_input(BenchmarkId::new("std", i), i, |b, i| {
                b.iter(|| push_back_std_small(*i))
            })
            .throughput(Throughput::Elements(*i as u64));
    }
    group.finish();

    let mut group = c.benchmark_group("push_back_big");
    for i in [20, 40, 60, 80].iter() {
        group
            .bench_with_input(BenchmarkId::new("crate", i), i, |b, i| {
                b.iter(|| push_back_crate_big(*i))
            })
            .throughput(Throughput::Elements(*i as u64));
        group
            .bench_with_input(BenchmarkId::new("std", i), i, |b, i| {
                b.iter(|| push_back_std_big(*i))
            })
            .throughput(Throughput::Elements(*i as u64));
    }
    group.finish();
}

criterion_group!(benches, bench_fibs);
criterion_main!(benches);
