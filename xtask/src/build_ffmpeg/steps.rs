//! Вызовы штатных build-инструментов библиотек; собственных shell-скриптов нет.

use super::{Build, execute};
use crate::XtaskResult;
use std::{fs, path::Path, process::Command};

/// Собирает статический программный VP9 encoder и устанавливает headers для FFmpeg.
pub(super) fn vpx(build: &Build) -> XtaskResult<()> {
    let directory = build.directory.join("vpx-build");
    let mut configure = build.configure("libvpx-1.16.0", "vpx-build")?;
    configure
        .arg(format!("--prefix={}", build.posix_path(&build.prefix)?))
        .args([
            "--enable-vp9",
            "--disable-vp8",
            "--disable-vp9-decoder",
            "--disable-examples",
            "--disable-tools",
            "--disable-docs",
            "--disable-unit-tests",
            "--disable-webm-io",
            "--disable-libyuv",
            "--disable-shared",
            "--enable-static",
        ]);
    if build.windows {
        configure.args(["--target=x86_64-win64-vs17", "--disable-static-msvcrt"]);
    } else {
        configure.arg("--enable-pic");
    }
    execute("конфигурация libvpx VP9", &mut configure)?;
    if build.windows {
        execute(
            "генерация MSVC-проекта libvpx",
            Command::new("make")
                .arg("NO_LAUNCH_DEVENV=1")
                .current_dir(&directory),
        )?;
        execute(
            "сборка libvpx MSVC",
            Command::new("MSBuild.exe")
                .arg("vpx.sln")
                .args(["/p:Configuration=Release", "/p:Platform=x64"])
                .arg(format!("/m:{}", build.jobs))
                .current_dir(&directory),
        )?;
        fs::create_dir_all(build.prefix.join("lib/pkgconfig"))
            .map_err(|error| error.to_string())?;
        fs::create_dir_all(build.prefix.join("include/vpx")).map_err(|error| error.to_string())?;
        fs::copy(
            directory.join("x64/Release/vpxmd.lib"),
            build.prefix.join("lib/vpx.lib"),
        )
        .map_err(|error| error.to_string())?;
        for entry in
            fs::read_dir(build.source("libvpx-1.16.0/vpx")).map_err(|error| error.to_string())?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().is_some_and(|value| value == "h") {
                fs::copy(
                    &path,
                    build
                        .prefix
                        .join("include/vpx")
                        .join(path.file_name().unwrap()),
                )
                .map_err(|error| error.to_string())?;
            }
        }
        let prefix = build.prefix.to_string_lossy().replace('\\', "/");
        fs::write(build.prefix.join("lib/pkgconfig/vpx.pc"), format!(
            "prefix={prefix}\nlibdir=${{prefix}}/lib\nincludedir=${{prefix}}/include\nName: vpx\nDescription: Static VP9 software encoder\nVersion: 1.16.0\nLibs: -L${{libdir}} -lvpx\nCflags: -I${{includedir}}\n"
        )).map_err(|error| error.to_string())?;
    } else {
        execute(
            "сборка libvpx",
            Command::new("make")
                .arg(format!("-j{}", build.jobs))
                .current_dir(&directory),
        )?;
        execute(
            "установка libvpx",
            Command::new("make").arg("install").current_dir(directory),
        )?;
    }
    Ok(())
}

/// Собирает статический dispatcher QSV, сохраняя GPU runtime системным.
pub(super) fn vpl(build: &Build) -> XtaskResult<()> {
    let source = build.source("libvpl-2.16.0");
    let directory = build.directory.join("vpl-build");
    let mut configure = Command::new("cmake");
    configure
        .arg("-S")
        .arg(source)
        .arg("-B")
        .arg(&directory)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", build.prefix.display()))
        .args([
            "-DCMAKE_INSTALL_LIBDIR=lib",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DBUILD_TESTS=OFF",
            "-DBUILD_EXAMPLES=OFF",
            "-DINSTALL_EXAMPLES=OFF",
        ]);
    if build.windows {
        configure.args([
            "-G",
            "Ninja",
            "-DCMAKE_C_COMPILER=cl",
            "-DCMAKE_CXX_COMPILER=cl",
        ]);
    }
    // Шаблон vpl.pc не перечисляет эти зависимости статического dispatcher.
    // Передаём их через Libs.private для проверки линковки и configure FFmpeg.
    let private_libraries = if build.windows {
        "-ladvapi32 -lole32"
    } else {
        "-lstdc++"
    };
    configure.arg(format!(
        "-DVPL_PKGCONFIG_PRIVATE_LIBS:STRING={private_libraries}"
    ));
    execute("конфигурация oneVPL", &mut configure)?;
    execute(
        "сборка oneVPL",
        Command::new("cmake").arg("--build").arg(&directory).args([
            "--config",
            "Release",
            "--parallel",
            &build.jobs,
        ]),
    )?;
    execute(
        "установка oneVPL",
        Command::new("cmake")
            .arg("--install")
            .arg(directory)
            .args(["--config", "Release"]),
    )?;
    verify_vpl_pkg_config(build)
}

fn verify_vpl_pkg_config(build: &Build) -> XtaskResult<()> {
    for arguments in [vec!["--modversion", "vpl"], vec!["--path", "vpl"]] {
        let mut command = build.pkg_config_command()?;
        command.args(&arguments);
        let output = command
            .output()
            .map_err(|error| format!("Не удалось запустить pkg-config для oneVPL: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "pkg-config не обнаружил установленную oneVPL ({}): {}",
                arguments.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        println!(
            "[FFmpeg] pkgconf {}: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }

    let mut cflags_command = build.pkg_config_command()?;
    cflags_command.args(["--cflags", "vpl"]);
    let cflags = checked_output("pkgconf --cflags vpl", &mut cflags_command)?;
    let mut libs_command = build.pkg_config_command()?;
    libs_command.args(["--static", "--libs", "vpl"]);
    let libraries = checked_output("pkgconf --static --libs vpl", &mut libs_command)?;
    println!("[FFmpeg] oneVPL compile flags: {cflags}");
    println!("[FFmpeg] oneVPL static link flags: {libraries}");

    let source = build.directory.join("vpl-probe.c");
    fs::write(
        &source,
        "#include <mfxvideo.h>\n#include <mfxdispatcher.h>\nint main(void) { mfxLoader loader = MFXLoad(); if (loader) MFXUnload(loader); return 0; }\n",
    )
    .map_err(|error| error.to_string())?;
    let output = build.directory.join(if build.windows {
        "vpl-probe.exe"
    } else {
        "vpl-probe"
    });
    let mut compile = if build.windows {
        windows_vpl_probe(&source, &output, &cflags, &libraries)?
    } else {
        let mut command = Command::new("cc");
        command.args(cflags.split_whitespace());
        command.arg(&source);
        command.args(libraries.split_whitespace());
        command.arg("-o").arg(&output);
        command
    };
    execute("проверка compile/link oneVPL через pkgconf", &mut compile)
}

fn windows_vpl_probe(
    source: &Path,
    output: &Path,
    cflags: &str,
    libraries: &str,
) -> XtaskResult<Command> {
    let mut command = Command::new("cl");
    command.args(["/nologo", "/MD"]);
    for flag in cflags.split_whitespace() {
        if let Some(path) = flag.strip_prefix("-I") {
            command.arg(format!("/I{}", windows_pkg_path(path)?));
        } else {
            command.arg(flag);
        }
    }
    command.arg(source).args(["/link", "/NOLOGO"]);
    for flag in libraries.split_whitespace() {
        if let Some(path) = flag.strip_prefix("-L") {
            command.arg(format!("/LIBPATH:{}", windows_pkg_path(path)?));
        } else if let Some(library) = flag.strip_prefix("-l") {
            command.arg(format!("{library}.lib"));
        } else {
            command.arg(flag);
        }
    }
    command.arg(format!("/OUT:{}", output.display()));
    Ok(command)
}

fn windows_pkg_path(path: &str) -> XtaskResult<String> {
    let output = Command::new("cygpath")
        .args(["-w", path])
        .output()
        .map_err(|error| format!("cygpath не преобразовал путь pkgconf {path}: {error}"))?;
    if !output.status.success() {
        return Err(format!("cygpath не преобразовал путь pkgconf {path}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn checked_output(stage: &str, command: &mut Command) -> XtaskResult<String> {
    let output = command
        .output()
        .map_err(|error| format!("{stage}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{stage}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Собирает libavcodec/libavformat с аппаратными VP9 и программным резервом.
pub(super) fn ffmpeg(build: &Build) -> XtaskResult<()> {
    let mut configure = build.configure("ffmpeg-8.0.1", "ffmpeg-build")?;
    configure
        .arg(format!(
            "--prefix={}",
            build.prefix.to_string_lossy().replace('\\', "/")
        ))
        .args([
            "--pkg-config-flags=--static",
            "--disable-everything",
            "--disable-autodetect",
            "--disable-programs",
            "--disable-doc",
            "--disable-debug",
            "--disable-network",
            "--disable-shared",
            "--enable-static",
            "--disable-x86asm",
            "--disable-avdevice",
            "--disable-avfilter",
            "--disable-swscale",
            "--disable-swresample",
            "--enable-avcodec",
            "--enable-avformat",
            "--enable-avutil",
            "--enable-muxer=webm",
            "--enable-bsf=vp9_superframe",
            "--enable-libvpl",
            "--enable-encoder=vp9_qsv",
            "--enable-libvpx",
            "--enable-encoder=libvpx_vp9",
        ]);
    if build.windows {
        configure.args([
            "--toolchain=msvc",
            "--arch=x86_64",
            "--target-os=win64",
            "--enable-d3d11va",
            "--enable-dxva2",
        ]);
    } else {
        configure.args([
            "--enable-vaapi",
            "--enable-libdrm",
            "--enable-encoder=vp9_vaapi",
            "--enable-pic",
        ]);
    }
    execute("конфигурация FFmpeg", &mut configure)?;
    let directory = build.directory.join("ffmpeg-build");
    execute(
        "сборка FFmpeg",
        Command::new("make")
            .arg(format!("-j{}", build.jobs))
            .current_dir(&directory),
    )?;
    execute(
        "установка FFmpeg",
        Command::new("make").arg("install").current_dir(directory),
    )
}
