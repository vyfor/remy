use crate::runtime::Runtime;
use crate::tracking::OwnerId;

pub fn capture_active() -> bool {
    let rt = Runtime::get();
    let focused = *rt.focused_owner.lock().unwrap();
    let f = rt.focus.lock().unwrap();
    let Some(cap_id) = f.active_capture else {
        return false;
    };
    let Some(focused) = focused else {
        return false;
    };
    f.captures
        .get(cap_id)
        .is_some_and(|cap| cap.entries.iter().any(|entry| entry.owner_id == focused))
}

pub fn with_capture<R>(id: &'static str, body: impl FnOnce() -> R) -> R {
    {
        let rt = Runtime::get();
        let mut f = rt.focus.lock().unwrap();
        f.capture_stack.push(id);
        f.active_capture = Some(id);
        f.captures.entry(id).or_default();
    }

    struct CaptureGuard;
    impl Drop for CaptureGuard {
        fn drop(&mut self) {
            let rt = Runtime::get();
            rt.focus.lock().unwrap().capture_stack.pop();
        }
    }

    let _guard = CaptureGuard;
    body()
}

pub(crate) fn active_capture_id() -> Option<&'static str> {
    Runtime::get().focus.lock().unwrap().active_capture
}

pub(crate) fn active_capture_has(owner_id: OwnerId) -> bool {
    let f = Runtime::get().focus.lock().unwrap();
    let Some(cap_id) = f.active_capture else {
        return false;
    };
    f.captures
        .get(cap_id)
        .is_some_and(|cap| cap.entries.iter().any(|entry| entry.owner_id == owner_id))
}

pub(crate) fn current_frame_capture_id() -> Option<&'static str> {
    Runtime::get()
        .focus
        .lock()
        .unwrap()
        .capture_stack
        .last()
        .copied()
}
