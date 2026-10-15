//! Регрессионные проверки действующего Nginx перед переносом edge-функций в Rust.

mod fixture;

use std::fmt::Debug;
use std::path::Path;

use crate::XtaskResult;
use fixture::Fixture;

const DEFAULT_IMAGE: &str = "nginx:alpine";

type Check = (&'static str, fn(&Fixture) -> XtaskResult<()>);

/// Проверяет HTTP-контракт на временных локальных Docker-контейнерах.
///
/// Не обращается к production-серверу и не изменяет существующие контейнеры.
pub(super) fn run(args: Vec<String>) -> XtaskResult<()> {
    let image = match args.as_slice() {
        [] => DEFAULT_IMAGE,
        [flag, image] if flag == "--image" && !image.is_empty() => image,
        _ => {
            return Err(
                "usage: cargo run -p xtask -- edge-contract [--image <local-nginx-image>]"
                    .to_owned(),
            );
        }
    };

    let fixture = Fixture::start(image)?;
    let checks: &[Check] = &[
        ("HTTP redirect", redirect),
        ("ACME challenge", acme_challenge),
        ("landing and security headers", landing),
        ("SPA fallback", spa_fallback),
        ("service worker", service_worker),
        ("asset caching", asset_caching),
        ("special documents", special_documents),
        ("API proxy", api_proxy),
        ("WebSocket upgrade headers", websocket_upgrade),
        ("HTTP request body limit", body_limit),
        ("auth rate limit configuration", auth_rate_limit),
    ];

    let mut failures = 0;
    for (name, check) in checks {
        match check(&fixture) {
            Ok(()) => println!("PASS {name}"),
            Err(error) => {
                failures += 1;
                eprintln!("FAIL {name}: {error}");
            }
        }
    }
    if failures != 0 {
        return Err(format!(
            "{failures} of {} edge contract checks failed",
            checks.len()
        ));
    }

    println!("All {} edge contract checks passed.", checks.len());
    Ok(())
}

fn expect<T: Debug + PartialEq>(actual: T, expected: T, context: &str) -> XtaskResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{context}: expected {expected:?}, got {actual:?}"))
    }
}

fn contains(value: &str, expected: &str, context: &str) -> XtaskResult<()> {
    if value.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "{context}: expected to contain {expected:?}, got {value:?}"
        ))
    }
}

fn redirect(fixture: &Fixture) -> XtaskResult<()> {
    let response = fixture.request(false, "GET", "/some/path?x=1", &[], None)?;
    expect(response.status, 301, "redirect status")?;
    expect(
        response.header("Location"),
        "https://cheenhub.ru/some/path?x=1",
        "redirect URL",
    )
}

fn acme_challenge(fixture: &Fixture) -> XtaskResult<()> {
    let response = fixture.request(
        false,
        "GET",
        "/.well-known/acme-challenge/test-token",
        &[],
        None,
    )?;
    expect(response.status, 200, "ACME status")?;
    expect(response.body, b"acme-contract".to_vec(), "ACME body")
}

fn landing(fixture: &Fixture) -> XtaskResult<()> {
    let response = fixture.request(true, "GET", "/", &[], None)?;
    expect(response.status, 200, "landing status")?;
    expect(
        response.body.as_slice(),
        b"landing-contract".as_slice(),
        "landing body",
    )?;
    expect(
        response.header("Cache-Control"),
        "no-cache",
        "landing cache",
    )?;
    contains(
        response.header("Strict-Transport-Security"),
        "max-age=63072000",
        "HSTS",
    )?;
    contains(
        response.header("Content-Security-Policy"),
        "script-src 'none'",
        "landing CSP",
    )?;
    expect(response.header("X-Frame-Options"), "DENY", "frame policy")?;
    expect(
        response.header("X-Content-Type-Options"),
        "nosniff",
        "content sniffing",
    )?;
    expect(
        response.header("Referrer-Policy"),
        "strict-origin-when-cross-origin",
        "referrer policy",
    )
}

fn spa_fallback(fixture: &Fixture) -> XtaskResult<()> {
    let response = fixture.request(true, "GET", "/settings/profile", &[], None)?;
    expect(response.status, 200, "SPA status")?;
    expect(
        response.body.as_slice(),
        b"spa-contract".as_slice(),
        "SPA body",
    )?;
    expect(response.header("Cache-Control"), "no-cache", "SPA cache")?;
    // В текущем Nginx внутренний переход к /index.html теряет CSP.
    // Это известный пробел, а не требование к будущему Rust edge.
    if response.header("Content-Security-Policy").is_empty() {
        eprintln!("WARN: SPA fallback has no CSP in current Nginx; Rust edge must fix this");
    }
    Ok(())
}

fn service_worker(fixture: &Fixture) -> XtaskResult<()> {
    let response = fixture.request(true, "GET", "/sw.js", &[], None)?;
    expect(response.status, 200, "service worker status")?;
    expect(
        response.header("Cache-Control"),
        "no-cache, no-store, must-revalidate",
        "service worker cache",
    )?;
    expect(
        response.header("Service-Worker-Allowed"),
        "/",
        "service worker scope",
    )
}

fn asset_caching(fixture: &Fixture) -> XtaskResult<()> {
    for (path, cache) in [
        ("/assets/test.js", "public, max-age=31536000, immutable"),
        ("/wasm/test.wasm", "public, max-age=31536000, immutable"),
        ("/icons/test.svg", "public, max-age=86400"),
        ("/screenshots/test.png", "public, max-age=86400"),
    ] {
        let response = fixture.request(true, "GET", path, &[], None)?;
        expect(response.status, 200, path)?;
        expect(response.header("Cache-Control"), cache, path)?;
    }
    Ok(())
}

fn special_documents(fixture: &Fixture) -> XtaskResult<()> {
    for (path, content_type) in [
        ("/robots.txt", "text/plain"),
        ("/sitemap.xml", "application/xml"),
        ("/manifest.webmanifest", "application/manifest+json"),
    ] {
        let response = fixture.request(true, "GET", path, &[], None)?;
        expect(response.status, 200, path)?;
        contains(response.header("Content-Type"), content_type, path)?;
        expect(response.header("Cache-Control"), "no-cache", path)?;
    }
    Ok(())
}

fn api_proxy(fixture: &Fixture) -> XtaskResult<()> {
    let response = fixture.request(true, "GET", "/api/example?key=value", &[], None)?;
    expect(response.status, 200, "API status")?;
    expect(
        response.body.as_slice(),
        b"mock-backend".as_slice(),
        "API body",
    )?;
    expect(
        response.header("X-Mock-URI"),
        "/api/example?key=value",
        "API URI",
    )?;
    expect(response.header("X-Mock-Host"), "cheenhub.ru", "API Host")?;
    expect(
        response.header("X-Mock-Forwarded-Proto"),
        "https",
        "API forwarded scheme",
    )?;
    expect(
        response.header("X-Content-Type-Options"),
        "nosniff",
        "API content sniffing",
    )
}

fn websocket_upgrade(fixture: &Fixture) -> XtaskResult<()> {
    for path in ["/api/realtime/ws", "/api/host-settings/logs/ws"] {
        let response = fixture.request(
            true,
            "GET",
            path,
            &["Upgrade: websocket", "Connection: Upgrade"],
            None,
        )?;
        expect(response.status, 200, path)?;
        expect(response.header("X-Mock-Upgrade"), "websocket", path)?;
        expect(response.header("X-Mock-Connection"), "upgrade", path)?;
    }
    Ok(())
}

fn body_limit(fixture: &Fixture) -> XtaskResult<()> {
    let body = fixture.oversized_body()?;
    let response = fixture.request(
        true,
        "POST",
        "/api/example",
        &["Expect: 100-continue"],
        Some(Path::new(&body)),
    )?;
    expect(response.status, 413, "oversized request status")
}

fn auth_rate_limit(_fixture: &Fixture) -> XtaskResult<()> {
    // Конфигурация нужна только при запуске проверки, а не при сборке xtask.
    let config = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../deploy/nginx/default.conf"),
    )
    .map_err(|error| format!("cannot read Nginx contract configuration: {error}"))?;
    let config = config.as_str();
    contains(config, "zone=cheenhub_auth:10m rate=5r/s", "auth rate")?;
    contains(
        config,
        "limit_req zone=cheenhub_auth burst=10 nodelay",
        "auth burst",
    )
}
