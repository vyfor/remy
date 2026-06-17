use std::hint::black_box;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use remy_core::runtime::{allocate_slot, apply_commits, next_slot_id, read_current, write_wake};

fn read(c: &mut Criterion) {
    let slot = next_slot_id();
    allocate_slot(slot, 42i32);

    c.bench_function("read", |b| {
        b.iter(|| {
            let _: &i32 = black_box(read_current(slot));
        });
    });
}

fn write(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");
    for n in [10, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let slot = next_slot_id();
                allocate_slot(slot, 0i32);
                for i in 0..n {
                    write_wake(slot, i as i32);
                }
                apply_commits();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, read, write);
criterion_main!(benches);
