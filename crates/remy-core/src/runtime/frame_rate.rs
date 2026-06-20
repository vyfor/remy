use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static FRAME_INTERVAL_MICROS: AtomicU32 = AtomicU32::new(16667);

pub fn set_frame_interval(interval: Duration) {
    let micros = interval.as_micros().min(u32::MAX as u128) as u32;
    FRAME_INTERVAL_MICROS.store(micros, Ordering::Relaxed);
}

pub fn set_frame_rate(fps: u32) {
    if fps == 0 {
        return;
    }
    let micros = 1000000u32 / fps;
    FRAME_INTERVAL_MICROS.store(micros, Ordering::Relaxed);
}

pub fn frame_interval() -> Duration {
    Duration::from_micros(FRAME_INTERVAL_MICROS.load(Ordering::Relaxed) as u64)
}
