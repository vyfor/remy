use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use remy_core::runtime::{
    allocate_slot, apply_commits, next_slot_id, read_current, track_read, write_wake,
};
use remy_core::tracking::{
    begin_render_tracking, end_render_tracking, set_dirty_slots, take_cleared_areas,
};
use remy_core::CachedView;
use remy_core::View;
use std::hint::black_box;

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

        let slots: Vec<_> = (0..n)
            .map(|_| {
                let slot = next_slot_id();
                allocate_slot(slot, 1337);
                slot
            })
            .collect();

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


fn cache(c: &mut Criterion) {
    let my_slot = next_slot_id();
    allocate_slot(my_slot, 1337);
    let other_slot = next_slot_id();
    allocate_slot(other_slot, 0);

    let mut group = c.benchmark_group("cache");

    let view = move |_buf: &mut Buffer, _area: Rect| {
        track_read(my_slot);
        let _ = read_current::<i32>(my_slot);
    };
    let cached = CachedView::new(12345, view);
    let area = Rect::new(0, 0, 191, 7);

    begin_render_tracking();

    // cache hit
    {
        let mut buf = Buffer::empty(area);
        set_dirty_slots(vec![]);
        cached.render(&mut buf, area);
        take_cleared_areas();

        set_dirty_slots(vec![]);

        group.bench_function("hit", |b| {
            b.iter(|| {
                cached.render(black_box(&mut buf), black_box(area));
            });
        });
    }

    // cache hit but some comps still dirty
    {
        let mut buf = Buffer::empty(area);
        set_dirty_slots(vec![]);
        cached.render(&mut buf, area);
        take_cleared_areas();

        set_dirty_slots(vec![other_slot]); // dirty but not ours

        group.bench_function("hit_dirty", |b| {
            b.iter(|| {
                cached.render(black_box(&mut buf), black_box(area));
            });
        });
    }

    // cache miss :(
    {
        let mut buf = Buffer::empty(area);
        set_dirty_slots(vec![]);
        cached.render(&mut buf, area);
        take_cleared_areas();

        set_dirty_slots(vec![my_slot]);

        group.bench_function("miss", |b| {
            b.iter_batched(
                || take_cleared_areas,
                |_| {
                    cached.render(&mut buf, area);
                },
                BatchSize::SmallInput,
            );
        });
    }

    end_render_tracking();
    group.finish();
}

criterion_group!(benches, read, write, cache);
criterion_main!(benches);
