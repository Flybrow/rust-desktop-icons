use super::*;
use crate::domain::{Fence, Look};

fn tmp(tag: &str) -> PathBuf {
    let d = env::temp_dir().join(format!("rdi-test-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn save_then_load() {
    let d = tmp("cfg");
    let mut c = Config::default();
    c.fences.push(Fence::new(7, "Apps", (100, 100, 360, 240), Look::default()));
    save_to(&d, &c).unwrap();
    assert_eq!(load_from(&d), (c.clone(), None));
    save_to(&d, &c).unwrap();
    fs::write(d.join("config.json"), "{broken").unwrap();
    let (restored, bad) = load_from(&d);
    assert_eq!(restored, c);
    assert_eq!(fs::read_to_string(bad.unwrap()).unwrap(), "{broken");
    assert!(!d.join("config.json").exists());
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn relocation_across_volumes() {
    let d = tmp("reloc");
    let src = d.join("dir");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub").join("f.txt"), "x").unwrap();
    relocate(&src, &d.join("copy")).unwrap();
    assert!(!src.exists());
    assert_eq!(fs::read_to_string(d.join("copy").join("sub").join("f.txt")).unwrap(), "x");
    assert!(relocate(&d.join("missing"), &d.join("x")).is_err() && !d.join("x").exists());
    fs::create_dir_all(d.join("desk")).unwrap();
    fs::write(d.join("copy").join("g.txt"), "y").unwrap();
    let (moved, errors) = evacuate(&d.join("copy"), &d.join("desk"));
    assert_eq!((moved.len(), errors.len()), (2, 0));
    assert!(!d.join("copy").exists());
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn move_with_unique_names() {
    let d = tmp("mv");
    let (a, b) = (d.join("a"), d.join("b"));
    fs::create_dir_all(&a).unwrap();
    fs::write(a.join("x.txt"), "1").unwrap();
    fs::create_dir_all(&b).unwrap();
    fs::write(b.join("x.txt"), "0").unwrap();
    let dst = move_into(&a.join("x.txt"), &b).unwrap();
    assert_eq!(dst, b.join("x (2).txt"));
    assert_eq!(fs::read_to_string(dst).unwrap(), "1");
    assert!(!a.join("x.txt").exists());
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn unique_names_skip_reserved_paths() {
    let d = tmp("uniq");
    fs::write(d.join("a.txt"), "").unwrap();
    let reserved = [d.join("a (2).txt")];
    let pick = unique_by(&d, OsStr::new("a.txt"), |p| p.exists() || reserved.contains(&p.to_path_buf()));
    assert_eq!(pick, d.join("a (3).txt"));
    assert_eq!(unique(&d, OsStr::new("b.txt")), d.join("b.txt"));
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn interrupted_saves_never_lose_the_config() {
    let d = tmp("crash");
    let mut c = Config::default();
    c.fences.push(Fence::new(3, "Docs", (0, 0, 300, 200), Look::default()));
    save_to(&d, &c).unwrap();
    fs::write(d.join("config.json.tmp"), "{half").unwrap();
    assert_eq!(load_from(&d).0, c);
    save_to(&d, &c).unwrap();
    fs::write(d.join("config.json"), "").unwrap();
    let (restored, bad) = load_from(&d);
    assert_eq!((restored, bad.is_some()), (c.clone(), true));
    for junk in ["", "null", "[]", "{\"fences\": 5}", "\u{feff}"] {
        let e = tmp("junk");
        fs::write(e.join("config.json"), junk).unwrap();
        let (cfg, bad) = load_from(&e);
        assert_eq!((cfg, bad.is_some()), (Config::default().sanitized(), true), "{junk:?}");
        fs::remove_dir_all(e).unwrap();
    }
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn moves_survive_awkward_files() {
    use std::os::windows::fs::OpenOptionsExt;
    let d = tmp("awk");
    let (a, b) = (d.join("a"), d.join("b"));
    fs::create_dir_all(&a).unwrap();
    let ro = a.join("lecture seule é😀.txt");
    fs::write(&ro, "r").unwrap();
    let mut perm = fs::metadata(&ro).unwrap().permissions();
    perm.set_readonly(true);
    fs::set_permissions(&ro, perm).unwrap();
    let moved = move_into(&ro, &b).unwrap();
    assert_eq!((fs::read_to_string(&moved).unwrap().as_str(), moved.file_name()), ("r", ro.file_name()));
    let deep = (0..14).fold(a.clone(), |p, i| p.join(format!("dossier-très-long-{i:02}")));
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("loin.txt"), "l").unwrap();
    assert!(deep.as_os_str().len() > 260);
    assert_eq!(fs::read_to_string(move_into(&deep.join("loin.txt"), &b).unwrap()).unwrap(), "l");
    let locked = a.join("locked.txt");
    fs::write(&locked, "k").unwrap();
    let hold = fs::OpenOptions::new().read(true).share_mode(0).open(&locked).unwrap();
    assert!(move_into(&locked, &b).is_err());
    assert!(locked.exists() && !b.join("locked.txt").exists());
    let (moved, errors) = evacuate(&a, &b);
    assert!(errors.iter().any(|e| e.starts_with("locked.txt")) && a.exists() && !moved.is_empty());
    drop(hold);
    let (_, errors) = evacuate(&a, &b);
    assert!(errors.is_empty() && !a.exists() && b.join("locked.txt").exists());
    for e in fs::read_dir(&b).unwrap().flatten() {
        let mut p = e.metadata().unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        p.set_readonly(false);
        let _ = fs::set_permissions(e.path(), p);
    }
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn never_moves_into_itself() {
    let d = tmp("self");
    let src = d.join("dir");
    fs::create_dir_all(src.join("sub")).unwrap();
    assert!(move_into(&src, &src.join("sub")).is_err());
    assert!(move_into(&src, &src).is_err());
    assert!(src.join("sub").exists() && fs::read_dir(src.join("sub")).unwrap().next().is_none());
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn folders_are_created_and_failures_named() {
    let d = tmp("ensure");
    fs::write(d.join("file"), "x").unwrap();
    let (ok, blocked) = (d.join("a").join("b"), d.join("file").join("c"));
    let errors = ensure(&[ok.clone(), blocked]);
    assert!(ok.is_dir() && errors.len() == 1 && errors[0].contains("file"), "{errors:?}");
    assert!(inside(&d.join("a").join("b"), &d.join("a")) && !inside(&d, &d.join("a")) && inside(&d, &d));
    fs::remove_dir_all(d).unwrap();
}
