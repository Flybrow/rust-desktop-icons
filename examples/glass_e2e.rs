mod support;

use serde_json::{Value, json};
use std::{env, fs, thread, time::Duration};
use support::dump::{dump, keep};
use support::{app, fences, point, quit, wait, work};

const BASE: u64 = 1;

struct Case {
    name: &'static str,
    look: Value,
    custom: &'static [&'static str],
    extra: Value,
    against: &'static str,
}

fn cases() -> Vec<Case> {
    let look = |name, key: &str, v: Value| Case { name, look: json!({ key: v }), custom: &[], extra: json!({}), against: "base" };
    let custom = |c: Case, custom| Case { custom, ..c };
    let shape = |name, extra: Value| Case { name, look: json!({}), custom: &[], extra, against: "base" };
    vec![
        custom(look("Color", "color", json!(0x10_40A0)), &["Color"]),
        custom(look("Alpha", "alpha", json!(255)), &["Alpha"]),
        custom(look("Icon", "icon", json!(32)), &["Icon"]),
        custom(look("Tint", "tint", json!(0xFF_0000)), &["Tint"]),
        custom(look("AutoHeight", "auto_height", json!(false)), &["AutoHeight"]),
        custom(look("Labels", "labels", json!(false)), &["Labels"]),
        custom(look("Align", "align", json!("Right")), &["Align"]),
        custom(look("LabelSize", "label_size", json!("Large")), &["LabelSize"]),
        custom(look("LabelColor", "label_color", json!(0x00_00FF)), &["LabelColor"]),
        custom(look("Spacing", "spacing", json!("Wide")), &["Spacing"]),
        custom(look("TitleAlign", "title_align", json!("Left")), &["TitleAlign"]),
        custom(look("TitleSize", "title_size", json!("Large")), &["TitleSize"]),
        custom(look("BarColor", "bar_color", json!(0xC0_3000)), &["BarColor"]),
        custom(look("BarHover", "bar_hover", json!(true)), &["BarHover"]),
        custom(look("Corner", "corner", json!("Larger")), &["Corner"]),
        Case {
            name: "Wallpaper",
            look: json!({"wallpaper": true, "alpha": 255}),
            custom: &["Wallpaper", "Alpha"],
            extra: json!({}),
            against: "Alpha",
        },
        shape("View List", json!({"view": "List"})),
        shape("View Details", json!({"view": "Details"})),
        shape("View Tiles", json!({"view": "Tiles"})),
        shape("View Content", json!({"view": "Content"})),
        shape("Vertical bar", json!({"vertical": true})),
        shape("Rolled", json!({"rolled": true})),
    ]
}

fn differs(a: &(i32, i32, Vec<u32>), b: &(i32, i32, Vec<u32>)) -> usize {
    if (a.0, a.1) != (b.0, b.1) {
        return usize::MAX;
    }
    let far = |p: u32, q: u32| (0..4).any(|c| ((p >> (8 * c)) & 0xFF).abs_diff((q >> (8 * c)) & 0xFF) > 24);
    a.2.iter().zip(&b.2).filter(|(p, q)| far(**p, **q)).count()
}

fn main() {
    let w = work();
    let root = env::temp_dir().join(format!("rdi-glass-{}", std::process::id()));
    let (data, shots) = (root.join("RustDesktopIcons"), root.join("dumps"));
    fs::create_dir_all(&shots).unwrap();
    let cases = cases();
    let ids: Vec<u64> = (BASE..=BASE + cases.len() as u64).collect();
    let (x, y) = (w.left + support::scale(40), w.top + support::scale(40));
    let fence = |id: u64, look: Value, extra: &Value, custom: Vec<&str>| {
        let tab = json!({"id": id, "title": "Glass", "view": extra.get("view")});
        let mut f = json!({"id": id, "x": x, "y": y, "w": support::scale(360), "h": support::scale(240), "tabs": [tab], "custom": custom});
        look.as_object().into_iter().flatten().for_each(|(k, v)| f[k] = v.clone());
        for k in ["vertical", "rolled"] {
            if let Some(v) = extra.get(k) {
                f[k] = v.clone();
            }
        }
        f
    };
    let mut list = vec![fence(BASE, json!({}), &json!({}), vec![])];
    for (i, c) in cases.iter().enumerate() {
        list.push(fence(BASE + 1 + i as u64, c.look.clone(), &c.extra, c.custom.to_vec()));
    }
    for id in &ids {
        let d = data.join("fences").join(id.to_string());
        fs::create_dir_all(&d).unwrap();
        ["alpha.txt", "beta.txt", "gamma.txt", "delta.txt"].iter().for_each(|f| fs::write(d.join(f), "x").unwrap());
    }
    let cfg = json!({"lang": "en", "auto_update": false, "auto_sort": false, "version": env!("CARGO_PKG_VERSION"),
        "look": {"glass": true}, "fences": list});
    fs::write(data.join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).unwrap();
    point((w.right - 4, w.bottom - 4));
    let mut child = app(&root, &[("RDI_DUMP", shots.to_str().unwrap())], &[]);
    let pid = child.id();
    let shown = wait(15, || fences(pid).len() == ids.len() && ids.iter().all(|id| dump(&shots, *id).is_some()));
    thread::sleep(Duration::from_millis(3000));
    let mut fails = vec![];
    support::verdict(shown, "every glass fence is drawn", &mut fails);
    let id_of = |name: &str| cases.iter().position(|c| c.name == name).map_or(BASE, |i| BASE + 1 + i as u64);
    for (i, Case { name, against, .. }) in cases.iter().enumerate() {
        let id = BASE + 1 + i as u64;
        let n = match (dump(&shots, id_of(against)), dump(&shots, id)) {
            (Some(b), Some(d)) => differs(&b, &d),
            _ => 0,
        };
        keep(&shots, id, name);
        support::verdict(
            n > 150,
            &format!("{name} changes a glass fence, compared with {against} ({} px)", if n == usize::MAX { -1 } else { n as i64 }),
            &mut fails,
        );
    }
    keep(&shots, BASE, "base");
    quit(&mut child);
    let _ = fs::remove_dir_all(&root);
    if fails.is_empty() {
        println!("GLASS E2E OK");
    } else {
        println!("{} check(s) failed", fails.len());
        std::process::exit(1);
    }
}
