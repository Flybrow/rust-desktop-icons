use crate::{app::with, domain::search};
use std::path::PathBuf;

pub struct Hit {
    pub path: PathBuf,
    pub name: String,
    pub icon: i32,
    pub place: String,
}

pub fn collect() -> Vec<Hit> {
    with(|a| {
        let mut out = vec![];
        for v in &a.views {
            let Some(f) = a.cfg.fence(v.id) else { continue };
            let own = v.browse.is_empty().then(|| (f.active().id, v.all()));
            for (tab, items) in own.into_iter().chain(v.cache.iter().map(|(t, i)| (*t, i.as_slice()))) {
                let at = search::place(f, tab);
                out.extend(items.iter().map(|it| Hit {
                    path: it.path.clone(),
                    name: String::from_utf16_lossy(&it.name),
                    icon: it.icon,
                    place: at.clone(),
                }));
            }
        }
        out
    })
    .unwrap_or_default()
}
