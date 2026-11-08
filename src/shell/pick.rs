use std::path::PathBuf;
use windows::{
    Win32::{
        Foundation::HWND,
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree},
        UI::Shell::{Common::COMDLG_FILTERSPEC, *},
    },
    core::{HSTRING, PCWSTR},
};

unsafe fn result(d: &IFileDialog, h: HWND) -> Option<PathBuf> {
    unsafe {
        d.Show(Some(h)).ok()?;
        let p = d.GetResult().ok()?.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s.map(PathBuf::from)
    }
}

unsafe fn json(d: &IFileDialog) -> Option<()> {
    let (name, spec) = (HSTRING::from("JSON"), HSTRING::from("*.json"));
    let filter = [COMDLG_FILTERSPEC { pszName: PCWSTR(name.as_ptr()), pszSpec: PCWSTR(spec.as_ptr()) }];
    unsafe {
        d.SetFileTypes(&filter).ok()?;
        d.SetDefaultExtension(&HSTRING::from("json")).ok()
    }
}

pub fn folder(h: HWND) -> Option<PathBuf> {
    unsafe {
        let d: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        d.SetOptions(d.GetOptions().ok()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        result(&d.into(), h)
    }
}

fn scripted() -> Option<PathBuf> {
    std::env::var_os("RDI_PICK").filter(|_| cfg!(debug_assertions)).map(PathBuf::from)
}

pub fn open_json(h: HWND) -> Option<PathBuf> {
    if let Some(p) = scripted() {
        return Some(p);
    }
    unsafe {
        let d: IFileDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        d.SetOptions(d.GetOptions().ok()? | FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST).ok()?;
        json(&d)?;
        result(&d, h)
    }
}

pub fn save_json(h: HWND, name: &str) -> Option<PathBuf> {
    if let Some(p) = scripted() {
        return Some(p);
    }
    unsafe {
        let d: IFileDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        d.SetOptions(d.GetOptions().ok()? | FOS_FORCEFILESYSTEM | FOS_OVERWRITEPROMPT).ok()?;
        json(&d)?;
        d.SetFileName(&HSTRING::from(name)).ok()?;
        result(&d, h)
    }
}
