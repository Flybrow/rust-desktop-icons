use super::Config;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const HIDDEN: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sys {
    RecycleBin,
    ThisPc,
    UserFiles,
    Network,
    ControlPanel,
}

impl Sys {
    pub const ALL: [Sys; 5] = [Sys::RecycleBin, Sys::ThisPc, Sys::UserFiles, Sys::Network, Sys::ControlPanel];

    pub fn clsid(self) -> &'static str {
        match self {
            Sys::RecycleBin => "{645FF040-5081-101B-9F08-00AA002F954E}",
            Sys::ThisPc => "{20D04FE0-3AEA-1069-A2D8-08002B30309D}",
            Sys::UserFiles => "{59031a47-3f72-44a7-89c5-5595fe6b30ee}",
            Sys::Network => "{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}",
            Sys::ControlPanel => "{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}",
        }
    }

    pub fn folder(self) -> &'static str {
        match self {
            Sys::RecycleBin => "shell:RecycleBinFolder",
            Sys::ThisPc => "shell:MyComputerFolder",
            Sys::UserFiles => "shell:UsersFilesFolder",
            Sys::Network => "shell:NetworkPlacesFolder",
            Sys::ControlPanel => "shell:ControlPanelFolder",
        }
    }

    pub fn path(self) -> PathBuf {
        PathBuf::from(format!("::{}", self.clsid()))
    }

    pub fn original(self, now: Option<u32>) -> Option<u32> {
        if self == Sys::RecycleBin && now == Some(HIDDEN) { None } else { now }
    }

    pub fn of(p: &Path) -> Option<Sys> {
        let s = p.to_str()?.strip_prefix("::")?;
        Sys::ALL.into_iter().find(|x| x.clsid().eq_ignore_ascii_case(s))
    }
}

impl Config {
    pub fn placed(&self) -> Vec<Sys> {
        Sys::ALL.into_iter().filter(|s| self.fences.iter().flat_map(|f| &f.tabs).any(|t| t.system.contains(s))).collect()
    }

    pub fn remember(&mut self, s: Sys, prev: Option<u32>) -> bool {
        let fresh = !self.desk_icons.iter().any(|(x, _)| *x == s);
        if fresh {
            self.desk_icons.push((s, prev));
        }
        fresh
    }

    pub fn forget(&mut self, s: Sys) -> Option<Option<u32>> {
        self.desk_icons.iter().position(|(x, _)| *x == s).map(|i| self.desk_icons.remove(i).1)
    }

    pub fn original(&self, s: Sys) -> Option<u32> {
        self.desk_icons.iter().find(|(x, _)| *x == s).and_then(|(_, p)| *p)
    }

    pub fn place(&mut self, s: Sys, tab: Option<u64>) {
        for t in self.fences.iter_mut().flat_map(|f| f.tabs.iter_mut()) {
            t.system.retain(|x| *x != s);
            if Some(t.id) == tab {
                t.system.push(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_icons_live_in_one_tab() {
        let c: Config = serde_json::from_str::<Config>(
            r#"{"fences":[{"id":1,"tabs":[{"id":1},{"id":5,"recycle":true}]},{"id":2,"tabs":[{"id":2,"recycle":true,"system":["ThisPc"]}]}]}"#,
        )
        .unwrap();
        let mut c = c.sanitized();
        let holders =
            |c: &Config, s: Sys| c.fences.iter().flat_map(|f| &f.tabs).filter(|t| t.system.contains(&s)).map(|t| t.id).collect::<Vec<_>>();
        assert_eq!((holders(&c, Sys::RecycleBin), holders(&c, Sys::ThisPc)), (vec![5], vec![2]));
        assert!(!serde_json::to_string(&c).unwrap().contains("recycle"));
        c.place(Sys::RecycleBin, Some(2));
        assert_eq!(c.tab(2, 2).unwrap().system, vec![Sys::ThisPc, Sys::RecycleBin]);
        assert!(c.tab(1, 5).unwrap().system.is_empty());
        c.place(Sys::ThisPc, None);
        assert_eq!(c.placed(), vec![Sys::RecycleBin]);
    }

    #[test]
    fn desktop_visibility_is_remembered_once() {
        let mut c = Config::default();
        assert!(c.remember(Sys::ThisPc, Some(0)));
        assert!(!c.remember(Sys::ThisPc, Some(1)));
        assert!(c.remember(Sys::Network, None));
        assert_eq!((c.original(Sys::ThisPc), c.original(Sys::Network), c.original(Sys::UserFiles)), (Some(0), None, None));
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.desk_icons, c.desk_icons);
        assert_eq!(c.forget(Sys::ThisPc), Some(Some(0)));
        assert_eq!((c.forget(Sys::ThisPc), c.forget(Sys::Network)), (None, Some(None)));
        assert!(c.desk_icons.is_empty());
    }

    #[test]
    fn virtual_paths_round_trip() {
        for s in Sys::ALL {
            assert_eq!(Sys::of(&s.path()), Some(s));
        }
        assert_eq!(Sys::of(Path::new("::{20d04fe0-3aea-1069-a2d8-08002b30309d}")), Some(Sys::ThisPc));
        assert_eq!(Sys::of(Path::new("C:\\Users")), None);
        assert_eq!(Sys::of(Path::new("::{00000000-0000-0000-0000-000000000000}")), None);
        assert_eq!(serde_json::to_string(&Sys::ThisPc).unwrap(), "\"ThisPc\"");
    }

    #[test]
    fn original_visibility() {
        assert_eq!(Sys::ThisPc.original(None), None);
        assert_eq!(Sys::ThisPc.original(Some(0)), Some(0));
        assert_eq!(Sys::ThisPc.original(Some(HIDDEN)), Some(HIDDEN));
        assert_eq!(Sys::RecycleBin.original(Some(HIDDEN)), None);
        assert_eq!(Sys::RecycleBin.original(Some(0)), Some(0));
    }
}
