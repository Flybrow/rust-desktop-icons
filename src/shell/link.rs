use crate::{store, win::wide_path};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use windows::{
    Win32::{
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, STGM_READ},
        UI::Shell::{IShellLinkW, ShellLink},
    },
    core::*,
};

pub fn link_target(p: &Path) -> Option<String> {
    let ext = p.extension()?.to_ascii_lowercase();
    if ext == "url" {
        let text = std::fs::read_to_string(p).ok()?;
        return text.lines().find_map(|l| l.strip_prefix("URL=")).map(str::to_string);
    }
    let (path, args) = shortcut(p)?;
    Some(format!("{path} {args}"))
}

pub fn folder_target(p: &Path) -> Option<PathBuf> {
    let (path, args) = shortcut(p)?;
    Some(PathBuf::from(path)).filter(|d| args.trim().is_empty() && d.is_dir())
}

fn shortcut(p: &Path) -> Option<(String, String)> {
    if !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")) {
        return None;
    }
    let text = |buf: &[u16]| String::from_utf16_lossy(crate::win::until_nul(buf));
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        link.cast::<IPersistFile>().ok()?.Load(PCWSTR(wide_path(p).as_ptr()), STGM_READ).ok()?;
        let mut path = [0u16; 1024];
        link.GetPath(&mut path, std::ptr::null_mut(), 0).ok()?;
        let mut args = [0u16; 1024];
        let _ = link.GetArguments(&mut args);
        Some((text(&path), text(&args)))
    }
}

fn existing(dir: &Path, name: &OsStr, same: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    store::candidates(dir, name).take_while(|p| p.exists()).find(|p| same(p))
}

pub fn link_into(target: &Path, dir: &Path) -> Result<PathBuf> {
    let ext = target.extension().map(|e| e.to_ascii_lowercase());
    if ext.as_ref().is_some_and(|e| e == "lnk" || e == "url") {
        let name = target.file_name().unwrap_or_default();
        let wanted = link_target(target);
        if let Some(dst) = existing(dir, name, |p| wanted.is_some() && link_target(p) == wanted) {
            return Ok(dst);
        }
        let dst = store::put_new(dir, name, |d| store::copy_new(target, d))?;
        crate::undo::created(&dst);
        return Ok(dst);
    }
    let name = format!("{}.lnk", target.file_name().unwrap_or_default().to_string_lossy());
    if let Some(dst) = existing(dir, name.as_ref(), |p| shortcut(p).is_some_and(|(t, _)| super::same_path(Path::new(&t), target))) {
        return Ok(dst);
    }
    let dst = store::unique(dir, name.as_ref());
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(PCWSTR(wide_path(target).as_ptr()))?;
        if let Some(parent) = target.parent() {
            link.SetWorkingDirectory(PCWSTR(wide_path(parent).as_ptr()))?;
        }
        link.cast::<IPersistFile>()?.Save(PCWSTR(wide_path(&dst).as_ptr()), true)?;
    }
    crate::undo::created(&dst);
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::same_path;
    use std::fs;
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

    #[test]
    fn links_and_copies_shortcuts() {
        let d = std::env::temp_dir().join(format!("rdi-test-lnk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        let target = d.join("app.txt");
        fs::write(&target, "x").unwrap();
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let out = d.join("out");
        fs::create_dir_all(&out).unwrap();
        let lnk = link_into(&target, &out).unwrap();
        assert_eq!(lnk, out.join("app.txt.lnk"));
        assert!(fs::metadata(&lnk).unwrap().len() > 0);
        assert_eq!(link_into(&target, &out).unwrap(), lnk);
        assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
        fs::create_dir_all(d.join("other")).unwrap();
        let twin = d.join("other").join("app.txt");
        fs::write(&twin, "y").unwrap();
        let second = link_into(&twin, &out).unwrap();
        assert_eq!(
            (second.clone(), link_into(&twin, &out).unwrap(), link_into(&target, &out).unwrap()),
            (out.join("app.txt (2).lnk"), second.clone(), lnk.clone())
        );
        let copies = d.join("copies");
        fs::create_dir_all(&copies).unwrap();
        let copied = link_into(&lnk, &copies).unwrap();
        assert_eq!((copied.clone(), link_into(&lnk, &copies).unwrap()), (copies.join("app.txt.lnk"), copies.join("app.txt.lnk")));
        let alt = d.join("alt").join("app.txt.lnk");
        fs::create_dir_all(d.join("alt")).unwrap();
        fs::copy(&second, &alt).unwrap();
        assert_eq!(link_into(&alt, &copies).unwrap(), copies.join("app.txt (2).lnk"));
        assert_eq!((fs::read_dir(&out).unwrap().count(), fs::read_dir(&copies).unwrap().count()), (2, 2));
        assert!(same_path(&out, Path::new(&out.to_string_lossy().to_uppercase())));
        assert!(target.exists());
        assert_eq!(folder_target(&lnk), None);
        let dir = d.join("folder");
        fs::create_dir_all(&dir).unwrap();
        let to_dir = link_into(&dir, &out).unwrap();
        assert!(folder_target(&to_dir).is_some_and(|t| same_path(&t, &dir)));
        assert_eq!(folder_target(&dir), None);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(folder_target(&to_dir), None);
        fs::remove_dir_all(d).unwrap();
    }
}
