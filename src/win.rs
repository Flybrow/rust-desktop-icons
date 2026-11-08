use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::{GetMonitorInfoW, HBITMAP, HBRUSH, HFONT, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect},
        System::{
            LibraryLoader::GetModuleHandleW,
            Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
        },
        UI::{
            Controls::{Dialogs::*, EM_LIMITTEXT, EM_SETSEL},
            HiDpi::{GetDpiForSystem, GetDpiForWindow},
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, GetKeyState, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VIRTUAL_KEY, VK_LBUTTON, VK_RBUTTON,
            },
            WindowsAndMessaging::*,
        },
    },
    core::*,
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

pub fn wide_path(p: &Path) -> Vec<u16> {
    p.as_os_str().encode_wide().chain([0]).collect()
}

pub fn inst() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).map(|m| HINSTANCE(m.0)).unwrap_or_default() }
}

pub fn window_rect(h: HWND) -> RECT {
    let mut r = RECT::default();
    unsafe { GetWindowRect(h, &mut r).ok() };
    r
}

pub fn client_rect(h: HWND) -> RECT {
    let mut r = RECT::default();
    unsafe { GetClientRect(h, &mut r).ok() };
    r
}

pub fn work_area(r: &RECT) -> RECT {
    let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    let _ = unsafe { GetMonitorInfoW(MonitorFromRect(r, MONITOR_DEFAULTTONEAREST), &mut mi) };
    mi.rcWork
}

pub fn work_area_at((x, y): (i32, i32)) -> RECT {
    work_area(&RECT { left: x, top: y, right: x + 1, bottom: y + 1 })
}

pub fn wheel(wp: WPARAM) -> i32 {
    (wp.0 >> 16) as i16 as i32
}

pub fn typed(wp: WPARAM) -> Option<char> {
    (wp.0 >= 0x20 && wp.0 != 0x7F).then(|| char::from_u32(wp.0 as u32)).flatten()
}

pub fn scale(h: HWND, v: i32) -> i32 {
    v * unsafe { GetDpiForWindow(h) }.max(96) as i32 / 96
}

pub const WM_MOUSELEAVE: u32 = 0x02A3;

pub fn xy(lp: LPARAM) -> (i32, i32) {
    (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32)
}

pub fn track_leave(h: HWND) {
    let mut tme = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: h, dwHoverTime: 0 };
    let _ = unsafe { TrackMouseEvent(&mut tme) };
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT::default();
    let _ = unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

pub fn key_down(k: VIRTUAL_KEY) -> bool {
    (unsafe { GetKeyState(k.0 as i32) }) < 0
}

pub fn held(k: VIRTUAL_KEY) -> bool {
    (unsafe { GetAsyncKeyState(k.0 as i32) }) < 0
}

pub fn buttons_down() -> bool {
    [VK_LBUTTON, VK_RBUTTON].into_iter().any(held)
}

fn qpc_ms(ticks: i64) -> f64 {
    let mut freq = 0i64;
    let _ = unsafe { QueryPerformanceFrequency(&mut freq) };
    ticks as f64 * 1000. / freq.max(1) as f64
}

pub fn now_ms() -> f64 {
    let mut t = 0i64;
    let _ = unsafe { QueryPerformanceCounter(&mut t) };
    qpc_ms(t)
}

pub fn desktop_at((x, y): (i32, i32)) -> bool {
    let root = unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) };
    let mut buf = [0u16; 32];
    let n = unsafe { GetClassNameW(root, &mut buf) } as usize;
    matches!(String::from_utf16_lossy(&buf[..n]).as_str(), "Progman" | "WorkerW")
}

pub fn stamp(seconds: bool) -> String {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    let day = format!("{:04}-{:02}-{:02} {:02}:{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute);
    if seconds { format!("{day}:{:02}", t.wSecond) } else { day }
}

pub fn sys_scale(v: i32) -> i32 {
    v * unsafe { GetDpiForSystem() }.max(96) as i32 / 96
}

pub fn register(class: PCWSTR, proc: WNDPROC) {
    register_with(class, proc, HBRUSH::default());
}

pub fn register_with(class: PCWSTR, proc: WNDPROC, brush: HBRUSH) {
    let wc = WNDCLASSW {
        style: CS_DBLCLKS,
        lpfnWndProc: proc,
        hInstance: inst(),
        lpszClassName: class,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
        hbrBackground: brush,
        ..Default::default()
    };
    unsafe { RegisterClassW(&wc) };
}

pub fn progman() -> Option<HWND> {
    unsafe { FindWindowW(w!("Progman"), None) }.ok()
}

pub fn keep_bottom(lp: LPARAM) {
    let p = unsafe { &mut *(lp.0 as *mut WINDOWPOS) };
    if (p.flags & SWP_NOZORDER).0 == 0 {
        p.hwndInsertAfter = HWND_BOTTOM;
    }
}

pub fn edit_popup(owner: HWND, r: RECT, text: &str, font: HFONT, center: bool) -> Option<HWND> {
    let align = if center { ES_CENTER } else { ES_LEFT };
    let style = WS_POPUP | WS_VISIBLE | WS_BORDER | WINDOW_STYLE((align | ES_AUTOHSCROLL) as u32);
    let (w, ht) = (r.right - r.left, r.bottom - r.top);
    let ex = WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
    unsafe {
        let e = CreateWindowExW(
            ex,
            w!("EDIT"),
            PCWSTR(wide(text).as_ptr()),
            style,
            r.left,
            r.top,
            w,
            ht,
            Some(owner),
            None,
            Some(inst()),
            None,
        )
        .ok()?;
        SendMessageW(e, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
        SendMessageW(e, EM_LIMITTEXT, Some(WPARAM(255)), None);
        SendMessageW(e, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let _ = SetForegroundWindow(e);
        let _ = SetFocus(Some(e));
        Some(e)
    }
}

pub fn take_text(e: HWND) -> String {
    let mut buf = [0u16; 260];
    let n = unsafe { GetWindowTextW(e, &mut buf) } as usize;
    unsafe { DestroyWindow(e).ok() };
    String::from_utf16_lossy(&buf[..n])
}

pub fn choose_color(h: HWND, init: u32) -> Option<u32> {
    let mut custom = [COLORREF(0); 16];
    let mut cc = CHOOSECOLORW {
        lStructSize: size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: h,
        rgbResult: COLORREF(init),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        ..Default::default()
    };
    unsafe { ChooseColorW(&mut cc) }.as_bool().then_some(cc.rgbResult.0 & 0xFF_FFFF)
}

pub fn msgbox(h: Option<HWND>, text: &str, style: MESSAGEBOX_STYLE, rtl: bool) -> MESSAGEBOX_RESULT {
    let flags = if rtl { style | MB_RTLREADING | MB_RIGHT } else { style };
    unsafe { MessageBoxW(h, PCWSTR(wide(text).as_ptr()), w!("Rust Desktop Icons"), flags | MB_SETFOREGROUND) }
}

pub fn menu() -> HMENU {
    unsafe { CreatePopupMenu().unwrap_or_default() }
}

pub fn item(m: HMENU, id: usize, text: &str, checked: bool) {
    let label = if checked { format!("{text}\t\u{2713}") } else { text.to_string() };
    unsafe { AppendMenuW(m, MF_STRING, id, PCWSTR(wide(&label).as_ptr())).ok() };
}

pub fn icon(m: HMENU, item: u32, by_position: bool, bmp: HBITMAP) {
    let info = MENUITEMINFOW { cbSize: size_of::<MENUITEMINFOW>() as u32, fMask: MIIM_BITMAP, hbmpItem: bmp, ..Default::default() };
    unsafe { SetMenuItemInfoW(m, item, by_position, &info).ok() };
}

pub fn submenu(m: HMENU, text: &str, child: HMENU) {
    unsafe { AppendMenuW(m, MF_POPUP, child.0 as usize, PCWSTR(wide(text).as_ptr())).ok() };
}

pub fn separator(m: HMENU) {
    unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()).ok() };
}

pub fn popup(h: HWND, m: HMENU, rtl: bool) -> usize {
    unsafe {
        let (x, y) = cursor_pos();
        let _ = SetForegroundWindow(h);
        let align = if rtl { TPM_LAYOUTRTL | TPM_RIGHTALIGN } else { TPM_LEFTALIGN };
        let r = TrackPopupMenu(m, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | align, x, y, None, h, None);
        let _ = DestroyMenu(m);
        let _ = PostMessageW(Some(h), WM_NULL, WPARAM(0), LPARAM(0));
        r.0 as usize
    }
}
