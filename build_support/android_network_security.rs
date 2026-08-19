use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use url::Url;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:3000";
const DIOXUS_ANDROID_RESOURCE_PATH: &str = "android/app/app/src/main/res/xml";
const DIOXUS_TARGET_NAME: &str = "cheen_hub";

/// Устанавливает ограниченную сетевую политику Android для host текущего API.
pub(crate) fn install() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").as_deref() != Some(OsStr::new("android")) {
        return;
    }

    println!("cargo:rerun-if-env-changed=CHEENHUB_BASE_URL");
    let configured_base_url = std::env::var("CHEENHUB_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
    let policy = network_security_config(&configured_base_url);
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo должен задать CARGO_MANIFEST_DIR"),
    );
    let target_dir = cargo_target_dir(&manifest_dir);
    let mut installed = false;

    for profile in ["debug", "release"] {
        let resource_dir = target_dir
            .join("dx")
            .join(DIOXUS_TARGET_NAME)
            .join(profile)
            .join(DIOXUS_ANDROID_RESOURCE_PATH);
        if !resource_dir.is_dir() {
            continue;
        }

        fs::create_dir_all(&resource_dir).unwrap_or_else(|error| {
            panic!(
                "не удалось создать каталог Android network security {}: {error}",
                resource_dir.display()
            )
        });
        let destination = resource_dir.join("network_security_config.xml");
        fs::write(&destination, &policy).unwrap_or_else(|error| {
            panic!(
                "не удалось установить Android network security policy {}: {error}",
                destination.display()
            )
        });
        installed = true;
    }

    if !installed {
        println!(
            "cargo:warning=Android network security policy не установлена: каталог ресурсов Dioxus ещё не создан"
        );
    }
}

fn network_security_config(base_url: &str) -> String {
    let url = Url::parse(base_url)
        .unwrap_or_else(|error| panic!("CHEENHUB_BASE_URL содержит некорректный URL: {error}"));
    let mut cleartext_hosts = vec!["127.0.0.1"];
    match url.scheme() {
        "http" => {
            let host = url
                .host_str()
                .unwrap_or_else(|| panic!("CHEENHUB_BASE_URL должен содержать host"));
            if host != "127.0.0.1" {
                cleartext_hosts.push(host);
            }
        }
        "https" => {}
        scheme => panic!("CHEENHUB_BASE_URL использует неподдерживаемую схему {scheme}"),
    }
    let domains = cleartext_hosts
        .into_iter()
        .map(|host| format!("        <domain includeSubdomains=\"false\">{host}</domain>"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<network-security-config>\n\
    <base-config cleartextTrafficPermitted=\"false\" />\n\
    <domain-config cleartextTrafficPermitted=\"true\">\n\
{domains}\n\
    </domain-config>\n\
</network-security-config>\n"
    )
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

#[cfg(test)]
mod tests {
    use super::network_security_config;

    #[test]
    fn permits_dioxus_loopback_and_configured_http_host() {
        let policy = network_security_config("http://192.168.2.2:3000");

        assert!(policy.contains("<domain includeSubdomains=\"false\">127.0.0.1</domain>"));
        assert!(policy.contains("<domain includeSubdomains=\"false\">192.168.2.2</domain>"));
        assert!(policy.contains("<base-config cleartextTrafficPermitted=\"false\" />"));
    }

    #[test]
    fn keeps_only_dioxus_loopback_for_https_api() {
        let policy = network_security_config("https://cheenhub.ru");

        assert!(policy.contains("<domain includeSubdomains=\"false\">127.0.0.1</domain>"));
        assert!(!policy.contains(">cheenhub.ru</domain>"));
        assert!(policy.contains("<base-config cleartextTrafficPermitted=\"false\" />"));
    }
}
