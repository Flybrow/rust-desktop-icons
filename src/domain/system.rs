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

#[cfg(test)]
mod tests {
    use super::*;

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
