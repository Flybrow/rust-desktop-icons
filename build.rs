use std::{env, fs, path::PathBuf, process::Command};

fn hook() {
    println!("cargo:rerun-if-changed=hook/src");
    println!("cargo:rerun-if-changed=hook/Cargo.toml");
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target = root.join("target").join("hook");
    let status = Command::new(env::var("CARGO").unwrap())
        .args(["build", "-p", "rdi-hook", "--release", "--target-dir"])
        .arg(&target)
        .current_dir(&root)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .status()
        .expect("cannot build rdi-hook");
    assert!(status.success(), "rdi-hook build failed");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("rdi_hook.dll");
    fs::copy(target.join("release").join("rdi_hook.dll"), out).expect("rdi_hook.dll missing");
}

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        hook();
        let mut r = winresource::WindowsResource::new();
        r.set_icon_with_id("assets/icon.ico", "1");
        r.set("ProductName", "Rust Desktop Icons");
        r.compile().expect("resource compilation failed");
    }
}
