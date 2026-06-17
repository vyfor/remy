use std::hint::black_box;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use remy_core::runtime::{allocate_slot, apply_commits, next_slot_id, read_current, track_read, write_wake};
use remy_core::tracking::{begin_render_tracking, end_render_tracking};

fn read(c: &mut Criterion) {
    let slot = next_slot_id();
    allocate_slot(slot, 1337);

    let mut group = c.benchmark_group("read");
    
    group.bench_function("base", |b| {
        b.iter(|| {
            let _: &i32 = black_box(read_current(slot));
        });
    });

    begin_render_tracking();
    group.bench_function("track", |b| {
        b.iter(|| {
            track_read(slot);
            let _: &i32 = black_box(read_current(slot));
        });
    });
    end_render_tracking();

    group.finish();
}

fn write(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");
    for n in [10, 100, 1_000] {
        group.throughput(Throughput::Elements(n as u64));
        
        let slots: Vec<_> = (0..n).map(|_| {
            let slot = next_slot_id();
            allocate_slot(slot, 1337);
            slot
        }).collect();

        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                for (i, &slot) in slots.iter().enumerate() {
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
