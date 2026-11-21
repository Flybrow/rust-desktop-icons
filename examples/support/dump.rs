use std::{env, fs, path::Path};

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
