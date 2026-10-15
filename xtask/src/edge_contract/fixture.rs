//! Изолированное окружение для проверки HTTP-контракта Nginx.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::XtaskResult;

/// Временный edge и mock backend в отдельной Docker-сети.
pub(super) struct Fixture {
    root: PathBuf,
    network: String,
    backend: String,
    edge: String,
    containers: Vec<String>,
    network_created: bool,
    http_port: u16,
    https_port: u16,
}

/// HTTP-ответ тестового сервера.
pub(super) struct Response {
    pub(super) status: u16,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl Response {
    /// Возвращает значение HTTP-заголовка без учёта регистра его имени.
    pub(super) fn header(&self, name: &str) -> &str {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map_or("", |(_, value)| value.as_str())
    }
}

impl Fixture {
    /// Создаёт временные ресурсы без изменения существующих контейнеров.
    pub(super) fn start(image: &str) -> XtaskResult<Self> {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("cannot locate repository root")?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let suffix = format!("{}-{nonce}", std::process::id());
        let root = env::temp_dir().join(format!("cheenhub-edge-{suffix}"));
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;

        let mut fixture = Self {
            root,
            network: format!("cheenhub-edge-{suffix}"),
            backend: format!("cheenhub-mock-{suffix}"),
            edge: format!("cheenhub-nginx-{suffix}"),
            containers: Vec::new(),
            network_created: false,
            http_port: 0,
            https_port: 0,
        };
        fixture.prepare(image, repo)?;
        Ok(fixture)
    }

    fn prepare(&mut self, image: &str, repo: &Path) -> XtaskResult<()> {
        self.create_files()?;

        // Готовим файловую структуру контейнера без bind mounts Docker Desktop.
        for (source, destination) in [
            ("static", "edge-root/usr/share/nginx/html"),
            ("certs", "edge-root/etc/letsencrypt"),
            ("certbot", "edge-root/var/www/certbot"),
        ] {
            let destination = self.root.join(destination);
            let parent = destination.parent().ok_or("invalid fixture destination")?;
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            fs::rename(self.root.join(source), destination)
                .map_err(|error| format!("cannot prepare fixture {source}: {error}"))?;
        }

        command("docker", &["image", "inspect", image])?;
        command("docker", &["network", "create", &self.network])?;
        self.network_created = true;

        self.containers.push(self.backend.clone());
        command(
            "docker",
            &[
                "create",
                "--name",
                &self.backend,
                "--network",
                &self.network,
                "--network-alias",
                "backend",
                image,
            ],
        )?;
        command(
            "docker",
            &[
                "cp",
                &self.root.join("mock.conf").to_string_lossy(),
                &format!("{}:/etc/nginx/conf.d/default.conf", self.backend),
            ],
        )?;
        command("docker", &["start", &self.backend])?;

        self.containers.push(self.edge.clone());
        command(
            "docker",
            &[
                "create",
                "--name",
                &self.edge,
                "--network",
                &self.network,
                "-p",
                "127.0.0.1::80",
                "-p",
                "127.0.0.1::443",
                image,
            ],
        )?;

        // docker cp работает через Docker API и не требует File Sharing.
        command(
            "docker",
            &[
                "cp",
                &format!("{}/.", self.root.join("edge-root").display()),
                &format!("{}:/", self.edge),
            ],
        )?;
        command(
            "docker",
            &[
                "cp",
                &repo.join("deploy/nginx/default.conf").to_string_lossy(),
                &format!("{}:/etc/nginx/conf.d/default.conf", self.edge),
            ],
        )?;
        command("docker", &["start", &self.edge])?;

        self.http_port = parse_port(&command("docker", &["port", &self.edge, "80/tcp"])?)?;
        self.https_port = parse_port(&command("docker", &["port", &self.edge, "443/tcp"])?)?;

        for _ in 0..40 {
            if matches!(self.request(false, "GET", "/", &[], None), Ok(r) if r.status == 301) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Err("temporary Nginx did not become ready".into())
    }

    fn create_files(&self) -> XtaskResult<()> {
        for (name, content) in [
            ("static/landing.html", "landing-contract"),
            ("static/index.html", "spa-contract"),
            ("static/robots.txt", "User-agent: *"),
            ("static/sitemap.xml", "<urlset/>"),
            ("static/sw.js", "// service worker"),
            ("static/manifest.webmanifest", "{}"),
            ("static/pwa-register.js", "// PWA"),
            ("static/offline.html", "offline-contract"),
            ("static/assets/test.js", "// asset"),
            ("static/wasm/test.wasm", "wasm fixture"),
            ("static/icons/test.svg", "<svg/>"),
            ("static/screenshots/test.png", "image fixture"),
            (
                "certbot/.well-known/acme-challenge/test-token",
                "acme-contract",
            ),
        ] {
            write(&self.root.join(name), content)?;
        }

        write(
            &self.root.join("mock.conf"),
            r#"
server {
    listen 3000;
    location / {
        add_header X-Mock-URI $request_uri always;
        add_header X-Mock-Host $http_host always;
        add_header X-Mock-Upgrade $http_upgrade always;
        add_header X-Mock-Connection $http_connection always;
        add_header X-Mock-Forwarded-Proto $http_x_forwarded_proto always;
        return 200 'mock-backend';
    }
}
"#,
        )?;

        let certs = self.root.join("certs/live/cheenhub.ru");
        fs::create_dir_all(&certs).map_err(|error| error.to_string())?;
        let key = certs.join("privkey.pem");
        let cert = certs.join("fullchain.pem");
        command(
            "openssl",
            &[
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-keyout",
                &key.to_string_lossy(),
                "-out",
                &cert.to_string_lossy(),
                "-subj",
                "/CN=cheenhub.ru",
            ],
        )?;
        Ok(())
    }

    /// Отправляет запрос только на loopback-порт временного Nginx.
    pub(super) fn request(
        &self,
        https: bool,
        method: &str,
        path: &str,
        headers: &[&str],
        body: Option<&Path>,
    ) -> XtaskResult<Response> {
        let port = if https {
            self.https_port
        } else {
            self.http_port
        };
        let scheme = if https { "https" } else { "http" };
        let resolve = format!("cheenhub.ru:{port}:127.0.0.1");
        let url = format!("{scheme}://cheenhub.ru:{port}{path}");
        let header_file = self.root.join("response-headers");
        let body_file = self.root.join("response-body");
        let mut args = vec![
            "--silent".to_owned(),
            "--show-error".to_owned(),
            "--insecure".to_owned(),
            "--http1.1".to_owned(),
            "--max-time".to_owned(),
            "10".to_owned(),
            "--noproxy".to_owned(),
            "*".to_owned(),
            "--resolve".to_owned(),
            resolve,
            "-H".to_owned(),
            "Host: cheenhub.ru".to_owned(),
            "-X".to_owned(),
            method.to_owned(),
            "-D".to_owned(),
            header_file.to_string_lossy().into_owned(),
            "-o".to_owned(),
            body_file.to_string_lossy().into_owned(),
            "-w".to_owned(),
            "%{http_code}".to_owned(),
        ];
        for header in headers {
            args.extend(["-H".to_owned(), (*header).to_owned()]);
        }
        if let Some(path) = body {
            args.extend(["--data-binary".to_owned(), format!("@{}", path.display())]);
        }
        args.push(url);

        let output = command("curl", &args.iter().map(String::as_str).collect::<Vec<_>>())?;
        let status = output
            .parse::<u16>()
            .map_err(|error| format!("invalid curl status {output:?}: {error}"))?;
        let headers = fs::read_to_string(&header_file).map_err(|error| error.to_string())?;
        let body = fs::read(&body_file).map_err(|error| error.to_string())?;
        Ok(Response {
            status,
            headers: parse_headers(&headers),
            body,
        })
    }

    /// Создаёт файл с телом запроса для проверки ограничения размера.
    pub(super) fn oversized_body(&self) -> XtaskResult<PathBuf> {
        let path = self.root.join("oversized-request");
        fs::File::create(&path)
            .and_then(|file| file.set_len(9 * 1024 * 1024 + 1))
            .map_err(|error| error.to_string())?;
        Ok(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for name in self.containers.iter().rev() {
            if let Err(error) = command("docker", &["rm", "-f", name]) {
                eprintln!("edge cleanup: {error}");
            }
        }
        if self.network_created
            && let Err(error) = command("docker", &["network", "rm", &self.network])
        {
            eprintln!("edge cleanup: {error}");
        }
        if let Err(error) = fs::remove_dir_all(&self.root) {
            eprintln!("edge cleanup: {}: {error}", self.root.display());
        }
    }
}

fn write(path: &Path, content: &str) -> XtaskResult<()> {
    fs::create_dir_all(path.parent().ok_or("file has no parent directory")?)
        .and_then(|()| fs::write(path, content))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn command(program: &str, args: &[&str]) -> XtaskResult<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub(super) fn parse_port(value: &str) -> XtaskResult<u16> {
    value
        .trim()
        .rsplit_once(':')
        .ok_or_else(|| format!("invalid Docker port: {value:?}"))?
        .1
        .parse()
        .map_err(|error| format!("invalid Docker port: {error}"))
}

pub(super) fn parse_headers(raw: &str) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    for line in raw.lines() {
        if line.starts_with("HTTP/") {
            headers.clear();
        } else if let Some((name, value)) = line.trim_end().split_once(':') {
            headers.push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }
    headers
}

#[cfg(test)]
mod tests;
