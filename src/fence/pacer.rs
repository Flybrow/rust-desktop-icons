use std::{sync::Mutex, thread};
use windows::Win32::{
    Foundation::{CloseHandle, HWND, LPARAM, WPARAM},
    Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, VREFRESH},
    Media::{timeBeginPeriod, timeEndPeriod},
    System::Threading::{
        CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, INFINITE, SetWaitableTimer, TIMER_ALL_ACCESS, WaitForSingleObject,
    },
    UI::WindowsAndMessaging::{PostMessageW, WM_APP},
};

pub(super) const WM_FRAME: u32 = WM_APP + 9;

struct State {
    fences: Vec<(isize, bool)>,
    running: bool,
}

static STATE: Mutex<State> = Mutex::new(State { fences: vec![], running: false });

fn period() -> i64 {
    let dc = unsafe { GetDC(None) };
    let hz = unsafe { GetDeviceCaps(Some(dc), VREFRESH) };
    unsafe { ReleaseDC(None, dc) };
    10_000_000 / hz.max(60) as i64
}

fn run() {
    let timer = unsafe { CreateWaitableTimerExW(None, None, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS.0) }
        .or_else(|_| unsafe { CreateWaitableTimerExW(None, None, 0, TIMER_ALL_ACCESS.0) });
    let period = period();
    let now = || (crate::win::now_ms() * 10_000.) as i64;
    let mut next = now() + period;
    unsafe { timeBeginPeriod(1) };
    loop {
        let due = -(next - now()).max(5_000);
        match &timer {
            Ok(t) if unsafe { SetWaitableTimer(*t, &due, 0, None, None, false) }.is_ok() => {
                unsafe { WaitForSingleObject(*t, INFINITE) };
            }
            _ => thread::sleep(std::time::Duration::from_micros(-due as u64 / 10)),
        }
        next = (next + period).max(now() + period / 2);
        let Ok(mut s) = STATE.lock() else { break };
        if s.fences.is_empty() {
            s.running = false;
            break;
        }
        for (h, pending) in s.fences.iter_mut().filter(|f| !f.1) {
            *pending = unsafe { PostMessageW(Some(HWND(*h as _)), WM_FRAME, WPARAM(0), LPARAM(0)) }.is_ok();
        }
    }
    unsafe { timeEndPeriod(1) };
    if let Ok(t) = timer {
        let _ = unsafe { CloseHandle(t) };
    }
}

pub(super) fn join(h: HWND) {
    let Ok(mut s) = STATE.lock() else { return };
    if !s.fences.iter().any(|f| f.0 == h.0 as isize) {
        s.fences.push((h.0 as isize, false));
    }
    if !std::mem::replace(&mut s.running, true) {
        if crate::perf::on() {
            crate::perf::mark("period", 0, period() as f64 / 10_000.);
        }
        thread::spawn(run);
    }
}

pub(super) fn leave(h: HWND) {
    if let Ok(mut s) = STATE.lock() {
        s.fences.retain(|f| f.0 != h.0 as isize);
    }
}

pub(super) fn ticked(h: HWND) {
    if let Ok(mut s) = STATE.lock() {
        s.fences.iter_mut().filter(|f| f.0 == h.0 as isize).for_each(|f| f.1 = false);
    }
}
