use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use remy_core::CachedView;
use remy_core::Rcx;
use remy_core::View;
use remy_core::runtime::{allocate_slot, next_slot_id, read_current, track_read};
use remy_core::tracking::{
    begin_render_tracking, end_render_tracking, set_dirty_slots, take_cleared_areas,
};
use std::hint::black_box;

fn cache(c: &mut Criterion) {
    let my_slot = next_slot_id();
    allocate_slot(my_slot, 1337);
    let other_slot = next_slot_id();
    allocate_slot(other_slot, 0);

    let mut group = c.benchmark_group("cache");

    let view = move |_rcx: Rcx, _buf: &mut Buffer, _area: Rect| {
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
        cached.render(Rcx::new(12345), &mut buf, area);
        take_cleared_areas();

        set_dirty_slots(vec![]);

        group.bench_function("hit", |b| {
            b.iter(|| {
                cached.render(Rcx::new(12345), black_box(&mut buf), black_box(area));
            });
        });
    }

    // cache hit but some comps still dirty
    {
        let mut buf = Buffer::empty(area);
        set_dirty_slots(vec![]);
        cached.render(Rcx::new(12345), &mut buf, area);
        take_cleared_areas();

        set_dirty_slots(vec![other_slot]); // dirty but not ours

        group.bench_function("hit_dirty", |b| {
            b.iter(|| {
                cached.render(Rcx::new(12345), black_box(&mut buf), black_box(area));
            });
        });
    }

    // cache miss :(
    {
        let mut buf = Buffer::empty(area);
        set_dirty_slots(vec![]);
        cached.render(Rcx::new(12345), &mut buf, area);
        take_cleared_areas();

        set_dirty_slots(vec![my_slot]);

        group.bench_function("miss", |b| {
            b.iter_batched(
                || take_cleared_areas,
                |_| {
                    cached.render(Rcx::new(12345), &mut buf, area);
                },
                BatchSize::SmallInput,
            );
        });
    }

    end_render_tracking();
    group.finish();
}

criterion_group!(benches, cache);
criterion_main!(benches);
