use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, AtomicI64, AtomicIsize, Ordering};
use once_cell::sync::Lazy;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Shell::{
    ITaskbarList3, TaskbarList, TBPFLAG, TBPF_NOPROGRESS, TBPF_NORMAL
};

struct TaskbarWrapper(ITaskbarList3);
unsafe impl Send for TaskbarWrapper {}
unsafe impl Sync for TaskbarWrapper {}

static TASKBAR_LIST: Lazy<Mutex<Option<TaskbarWrapper>>> = Lazy::new(|| Mutex::new(None));

// --- W-8 fix: replace static mut with atomics so set_progress_state /
// set_progress_value called from download threads are race-free. ---
static TASKBAR_HWND: AtomicIsize = AtomicIsize::new(0);
// TBPFLAG is i32 under the hood
static CURRENT_STATE: AtomicI32 = AtomicI32::new(TBPF_NOPROGRESS.0);
static CURRENT_VALUE: AtomicI64 = AtomicI64::new(0);

fn get_hwnd() -> HWND {
    HWND(TASKBAR_HWND.load(Ordering::Relaxed) as *mut _)
}

pub fn init(hwnd: HWND) {
    TASKBAR_HWND.store(hwnd.0 as isize, Ordering::Release);
    if let Ok(taskbar) = unsafe { CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) } {
        let _ = unsafe { taskbar.SetProgressState(hwnd, TBPF_NOPROGRESS) };
        if let Ok(mut guard) = TASKBAR_LIST.lock() {
            *guard = Some(TaskbarWrapper(taskbar));
        }
    }
}

pub fn set_progress_state(state: TBPFLAG) {
    if CURRENT_STATE.load(Ordering::Relaxed) == state.0 { return; }
    CURRENT_STATE.store(state.0, Ordering::Relaxed);
    let hwnd = get_hwnd();
    if let Ok(guard) = TASKBAR_LIST.lock() {
        if let Some(wrapper) = guard.as_ref() {
            let _ = unsafe { wrapper.0.SetProgressState(hwnd, state) };
        }
    }
}

pub fn set_progress_value(completed: u64, total: u64) {
    let completed_i = completed as i64;
    if CURRENT_VALUE.load(Ordering::Relaxed) == completed_i
        && CURRENT_STATE.load(Ordering::Relaxed) == TBPF_NORMAL.0
    {
        return;
    }
    CURRENT_VALUE.store(completed_i, Ordering::Relaxed);
    CURRENT_STATE.store(TBPF_NORMAL.0, Ordering::Relaxed);
    let hwnd = get_hwnd();
    if let Ok(guard) = TASKBAR_LIST.lock() {
        if let Some(wrapper) = guard.as_ref() {
            let _ = unsafe { wrapper.0.SetProgressState(hwnd, TBPF_NORMAL) };
            let _ = unsafe { wrapper.0.SetProgressValue(hwnd, completed, total) };
        }
    }
}