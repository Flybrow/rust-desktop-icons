use super::Res;
use std::{
    env,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

fn chain(first: &str, exe: &Path) -> String {
    format!("/v:on /c \"{first} & set rc=!errorlevel!& start \"\" /b \"{}\" {} & exit !rc!\"", exe.display(), crate::shell::RELAUNCH_ARG)
}

pub fn install(msi: &Path) -> Res<u32> {
    if cfg!(debug_assertions) {
        crate::report::log(&format!("debug build: not installing {}", msi.display()));
        return Ok(0);
    }
    let sys = env::var_os("SystemRoot").map(PathBuf::from).ok_or("no SystemRoot")?;
    let msiexec = sys.join("System32").join("msiexec.exe");
    let first = format!("\"{}\" /i \"{}\" /qn /norestart", msiexec.display(), msi.display());
    let line = chain(&first, &env::current_exe()?);
    let status = Command::new(sys.join("System32").join("cmd.exe")).raw_arg(line).creation_flags(0x0800_0000).status()?;
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
        fs::create_dir_all(dir.join("with space")).unwrap();
        let (bat, marker) = (dir.join("with space").join("app.bat"), dir.join("ran.txt"));
        fs::write(&bat, format!("@echo %1> \"{}\"\r\n@exit\r\n", marker.display())).unwrap();
        let sys = PathBuf::from(env::var_os("SystemRoot").unwrap()).join("System32");
        let first = format!("\"{}\" /c exit 3010", sys.join("cmd.exe").display());
        let status = Command::new(sys.join("cmd.exe")).raw_arg(chain(&first, &bat)).creation_flags(0x0800_0000).status().unwrap();
        assert_eq!(status.code(), Some(3010));
        let ran = (0..50).find_map(|_| {
            std::thread::sleep(Duration::from_millis(100));
            fs::read_to_string(&marker).ok().filter(|s| !s.is_empty())
        });
        assert_eq!(ran.as_deref().map(str::trim), Some(crate::shell::RELAUNCH_ARG));
        let _ = fs::remove_dir_all(&dir);
    }
}
