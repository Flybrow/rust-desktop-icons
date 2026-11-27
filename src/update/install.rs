use super::Res;
use std::{
    env, io,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

fn chain(first: &str) -> String {
    format!("/v:on /c \"{first} & set rc=!errorlevel!& start \"\" /b \"!RDI_EXE!\" {} & exit !rc!\"", crate::shell::RELAUNCH_ARG)
}

fn run(first: &str, vars: &[(&str, &Path)]) -> io::Result<ExitStatus> {
    let sys = env::var_os("SystemRoot").map(PathBuf::from).ok_or(io::ErrorKind::NotFound)?.join("System32");
    Command::new(sys.join("cmd.exe")).raw_arg(chain(first)).envs(vars.iter().copied()).creation_flags(0x0800_0000).status()
}

pub fn install(msi: &Path) -> Res<u32> {
    if cfg!(debug_assertions) {
        crate::report::log(&format!("debug build: not installing {}", msi.display()));
        return Ok(0);
    }
    let sys = env::var_os("SystemRoot").map(PathBuf::from).ok_or("no SystemRoot")?;
    let msiexec = sys.join("System32").join("msiexec.exe");
    let exe = env::current_exe()?;
    let first = "\"!RDI_MSIEXEC!\" /i \"!RDI_MSI!\" /qn /norestart";
    let status = run(first, &[("RDI_MSIEXEC", &msiexec), ("RDI_MSI", msi), ("RDI_EXE", &exe)])?;
    Ok(status.code().unwrap_or(-1) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::Duration};

    #[test]
    fn installer_chain_relaunches_and_keeps_the_exit_code() {
        let dir = env::temp_dir().join(format!("rdi-chain-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let odd = dir.join("Gamer! with space");
        fs::create_dir_all(&odd).unwrap();
        let (bat, marker) = (odd.join("app!.bat"), dir.join("ran.txt"));
        fs::write(&bat, format!("@echo %1> \"{}\"{r}@exit{r}", marker.display(), r = "\r\n")).unwrap();
        let cmd = PathBuf::from(env::var_os("SystemRoot").unwrap()).join("System32").join("cmd.exe");
        let status = run("\"!RDI_CMD!\" /c exit 3010", &[("RDI_CMD", &cmd), ("RDI_EXE", &bat)]).unwrap();
        assert_eq!(status.code(), Some(3010));
        let ran = (0..50).find_map(|_| {
            std::thread::sleep(Duration::from_millis(100));
            fs::read_to_string(&marker).ok().filter(|s| !s.is_empty())
        });
        assert_eq!(ran.as_deref().map(str::trim), Some(crate::shell::RELAUNCH_ARG));
        let _ = fs::remove_dir_all(&dir);
    }
}
