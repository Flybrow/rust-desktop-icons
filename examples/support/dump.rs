use std::{collections::HashSet, env, fs, path::Path};

pub type Dump = (i32, i32, Vec<u32>);

pub fn dump(dir: &Path, id: u64) -> Option<(i32, i32, Vec<u32>)> {
    let b = fs::read(dir.join(format!("{id}.bin"))).ok()?;
    let w = i32::from_le_bytes(b.get(..4)?.try_into().ok()?);
    let h = i32::from_le_bytes(b.get(4..8)?.try_into().ok()?);
    Some((w, h, b[8..].as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect()))
}

pub fn region(dir: &Path, id: u64, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> Vec<u32> {
    dump(dir, id).map_or(vec![], |(w, _, px)| {
        (y0..y1).flat_map(|y| (x0..x1).map(move |x| (y * w + x) as usize)).filter_map(|i| px.get(i).copied()).collect()
    })
}

pub fn keep(dir: &Path, id: u64, name: &str) {
    if let Ok(to) = env::var("KEEP_DUMPS") {
        let _ = fs::create_dir_all(&to);
        let _ = fs::copy(dir.join(format!("{id}.bin")), Path::new(&to).join(format!("{name}.bin")));
    }
}

pub fn alpha((w, _, px): &Dump, x: i32, y: i32) -> u32 {
    px.get((y * w + x) as usize).map_or(0, |p| p >> 24)
}

pub fn bands((w, _, px): &Dump, (x0, x1): (i32, i32), (y0, y1): (i32, i32), refx: i32) -> usize {
    let differs = |a: u32, b: u32| (0..4).any(|c| ((a >> (8 * c)) & 0xFF).abs_diff((b >> (8 * c)) & 0xFF) > 40);
    let px = |x: i32, y: i32| px.get((y * w + x) as usize).copied().unwrap_or(0);
    let rows: Vec<bool> = (y0..y1).map(|y| (x0..x1).any(|x| differs(px(x, y), px(refx, y)))).collect();
    rows.iter().enumerate().filter(|&(i, r)| *r && (i == 0 || !rows[i - 1])).count()
}

pub fn colors((w, _, px): &Dump, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> usize {
    (y0..y1).flat_map(|y| (x0..x1).map(move |x| (y * w + x) as usize)).filter_map(|i| px.get(i)).collect::<HashSet<_>>().len()
}

pub fn bar(d: &Dump, t: i32, (x0, x1): (i32, i32)) -> Vec<u32> {
    (0..t).flat_map(|y| (x0..x1).map(move |x| (y * d.0 + x) as usize)).filter_map(|i| d.2.get(i).copied()).collect()
}
