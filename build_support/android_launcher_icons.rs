use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

const DIOXUS_ANDROID_RESOURCE_PATH: &str = "android/app/app/src/main/res";
const DIOXUS_TARGET_NAME: &str = "cheen_hub";

/// Заменяет сгенерированные Dioxus launcher icons проектными Android-ресурсами.
pub(crate) fn install() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").as_deref() != Some(OsStr::new("android")) {
        return;
    }

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo должен задать CARGO_MANIFEST_DIR"),
    );
    let source = manifest_dir.join("android/res");
    println!("cargo:rerun-if-changed={}", source.display());

    let target_dir = cargo_target_dir(&manifest_dir);
    let mut installed = false;
    for profile in ["debug", "release"] {
        let destination = target_dir
            .join("dx")
            .join(DIOXUS_TARGET_NAME)
            .join(profile)
            .join(DIOXUS_ANDROID_RESOURCE_PATH);
        let generated_icon = destination.join("mipmap-anydpi-v26/ic_launcher.xml");
        println!("cargo:rerun-if-changed={}", generated_icon.display());

        if destination.is_dir() {
            copy_directory(&source, &destination);
            installed = true;
        }
    }

    if !installed {
        println!(
            "cargo:warning=Launcher icons Android не установлены: каталог ресурсов Dioxus ещё не создан"
        );
    }
}

fn cargo_target_dir(manifest_dir: &Path) -> PathBuf {
    if let Some(configured) = std::env::var_os("CARGO_TARGET_DIR") {
        let configured = PathBuf::from(configured);
        return if configured.is_absolute() {
            configured
        } else {
            manifest_dir.join("../..").join(configured)
        };
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("Cargo должен задать OUT_DIR"));
    let build_dir = out_dir
        .ancestors()
        .find(|path| path.file_name() == Some(OsStr::new("build")))
        .expect("Android OUT_DIR должен содержать каталог build");
    build_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("Android OUT_DIR должен находиться внутри target/<triple>/<profile>")
        .to_path_buf()
}

fn copy_directory(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source)
        .unwrap_or_else(|error| panic!("не удалось прочитать {}: {error}", source.display()))
    {
        let entry = entry.unwrap_or_else(|error| {
            panic!("не удалось прочитать запись в {}: {error}", source.display())
        });
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path).unwrap_or_else(|error| {
                panic!(
                    "не удалось создать каталог Android-ресурсов {}: {error}",
                    destination_path.display()
                )
            });
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap_or_else(|error| {
                panic!(
                    "не удалось установить Android-ресурс {} как {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            });
        }
    }
}
