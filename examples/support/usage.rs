use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX},
        Threading::{
            GR_GDIOBJECTS, GR_USEROBJECTS, GetGuiResources, GetProcessHandleCount, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        },
    },
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Usage {
    pub gdi: u32,
    pub user: u32,
    pub handles: u32,
    pub private_mb: f64,
}

pub fn usage(pid: u32) -> Option<Usage> {
    let h = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }.ok()?;
    let mut handles = 0;
    let mut mem = PROCESS_MEMORY_COUNTERS_EX { cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32, ..Default::default() };
    unsafe {
        let _ = GetProcessHandleCount(h, &mut handles);
        let ok = GetProcessMemoryInfo(h, &mut mem as *mut _ as *mut PROCESS_MEMORY_COUNTERS, mem.cb).is_ok();
        let u = Usage {
            gdi: GetGuiResources(h, GR_GDIOBJECTS),
            user: GetGuiResources(h, GR_USEROBJECTS),
            handles,
            private_mb: mem.PrivateUsage as f64 / 1048576.0,
        };
        let _ = CloseHandle(h);
        ok.then_some(u)
    }
}
