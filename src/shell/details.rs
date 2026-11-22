use crate::{
    domain::order::Meta,
    win::{until_nul, wide_path},
};
use std::path::Path;
use windows::{
    Win32::{
        Foundation::{FILETIME, SYSTEMTIME},
        Globalization::{DATE_SHORTDATE, GetDateFormatEx, GetTimeFormatEx, TIME_NOSECONDS},
        Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL},
        System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime},
        UI::Shell::{SHFILEINFOW, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW, StrFormatByteSizeW, StrFormatKBSizeW},
    },
    core::PCWSTR,
};

#[derive(Clone, Debug, Default, PartialEq, Hash)]
pub struct Info {
    pub modified: Vec<u16>,
    pub created: Vec<u16>,
    pub kind: Vec<u16>,
    pub size: Vec<u16>,
    pub bytes: Vec<u16>,
}

fn date(secs: u64) -> Option<Vec<u16>> {
    let t = (secs.checked_add(11_644_473_600)?).checked_mul(10_000_000)?;
    let ft = FILETIME { dwLowDateTime: t as u32, dwHighDateTime: (t >> 32) as u32 };
    let (mut utc, mut local) = (SYSTEMTIME::default(), SYSTEMTIME::default());
    unsafe { FileTimeToSystemTime(&ft, &mut utc).ok()? };
    unsafe { SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).ok()? };
    let (mut d, mut tm) = ([0u16; 80], [0u16; 40]);
    unsafe { GetDateFormatEx(PCWSTR::null(), DATE_SHORTDATE, Some(&local), None, Some(&mut d), None) };
    unsafe { GetTimeFormatEx(PCWSTR::null(), TIME_NOSECONDS, Some(&local), None, Some(&mut tm)) };
    let mut out = until_nul(&d).to_vec();
    out.push(' ' as u16);
    out.extend_from_slice(until_nul(&tm));
    Some(out)
}

fn kind(p: &Path, dir: bool) -> Vec<u16> {
    let (mut i, w) = (SHFILEINFOW::default(), wide_path(p));
    let attrs = if dir { FILE_ATTRIBUTE_DIRECTORY } else { FILE_ATTRIBUTE_NORMAL };
    let flags = SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES;
    unsafe { SHGetFileInfoW(PCWSTR(w.as_ptr()), attrs, Some(&mut i), size_of::<SHFILEINFOW>() as u32, flags) };
    until_nul(&i.szTypeName).to_vec()
}

fn sized(m: &Meta, f: unsafe fn(i64, &mut [u16]) -> windows::core::PWSTR) -> Vec<u16> {
    if m.dir {
        return vec![];
    }
    let mut buf = [0u16; 64];
    unsafe { f(m.size as i64, &mut buf) };
    until_nul(&buf).to_vec()
}

pub fn info(p: &Path, m: &Meta) -> Info {
    let when = |s: u64| (s > 0).then(|| date(s)).flatten().unwrap_or_default();
    Info {
        modified: when(m.modified),
        created: when(m.created),
        kind: kind(p, m.dir),
        size: sized(m, |n, b| unsafe { StrFormatKBSizeW(n, b) }),
        bytes: sized(m, |n, b| unsafe { StrFormatByteSizeW(n, b) }),
    }
}
