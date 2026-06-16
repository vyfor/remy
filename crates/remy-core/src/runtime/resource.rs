use std::sync::atomic::{AtomicBool, AtomicU64};

use super::Runtime;

pub fn bump_resource_gen(resource_id: u32) -> u64 {
    let rt = Runtime::get();
    let entry = rt
        .resource_gens
        .entry(resource_id)
        .or_insert(AtomicU64::new(0));
    entry.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
}

pub fn current_resource_gen(resource_id: u32) -> u64 {
    let rt = Runtime::get();
    rt.resource_gens
        .get(&resource_id)
        .map(|g| g.load(std::sync::atomic::Ordering::SeqCst))
        .unwrap_or(0)
}

pub fn mark_resource_fetched(resource_id: u32) {
    let rt = Runtime::get();
    rt.resource_fetched
        .entry(resource_id)
        .or_insert(AtomicBool::new(false))
        .store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn has_resource_fetched(resource_id: u32) -> bool {
    let rt = Runtime::get();
    rt.resource_fetched
        .get(&resource_id)
        .map(|b| b.load(std::sync::atomic::Ordering::SeqCst))
        .unwrap_or(false)
}
