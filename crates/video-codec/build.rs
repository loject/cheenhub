//! Дополнительные зависимости статических аппаратных backend FFmpeg.

use std::{env, path::PathBuf};

fn main() {
    let prefix =
        PathBuf::from(env::var_os("FFMPEG_DIR").expect("FFMPEG_DIR задаётся в .cargo/config.toml"));
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    println!(
        "cargo:rerun-if-changed={}",
        prefix.join("complete").display()
    );
    assert!(
        prefix.join("complete").exists(),
        "Сначала выполните cargo make build-ffmpeg"
    );
    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=vpl");
    println!("cargo:rustc-link-lib=static=vpx");
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let libraries: &[&str] = if target == "windows" {
        &[
            "d3d11", "dxva2", "dxgi", "cfgmgr32", "advapi32", "ole32", "user32", "bcrypt", "psapi",
            "shell32", "mfplat", "mfuuid", "strmiids", "oleaut32",
        ]
    } else {
        &["va-drm", "va", "drm", "stdc++", "m", "dl", "pthread"]
    };
    for library in libraries {
        println!("cargo:rustc-link-lib={library}");
    }
}
