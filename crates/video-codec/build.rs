//! Дополнительные зависимости статических аппаратных backend FFmpeg.

use std::{env, path::PathBuf};

fn main() {
    let prefix =
        PathBuf::from(env::var_os("FFMPEG_DIR").expect("FFMPEG_DIR задаётся в .cargo/config.toml"));
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let (file_prefix, extension) = match (target.as_str(), arch.as_str()) {
        ("windows", "x86_64") => ("", "lib"),
        ("linux", "x86_64") => ("lib", "a"),
        _ => panic!("Готовый пакет FFmpeg не поддерживает {target}/{arch}"),
    };
    let lib = prefix.join("lib");
    for name in ["avcodec", "avformat", "avutil", "vpl", "vpx"] {
        let archive = lib.join(format!("{file_prefix}{name}.{extension}"));
        assert!(
            archive.is_file(),
            "Отсутствует статическая библиотека {}",
            archive.display()
        );
        println!("cargo:rerun-if-changed={}", archive.display());
    }
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=vpl");
    println!("cargo:rustc-link-lib=static=vpx");
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
