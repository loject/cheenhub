//! Согласование параметров PostgreSQL с существующим DATABASE_URL.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use getrandom::SysRng;
use rand_core::{Rng, UnwrapErr};
use url::Url;

use super::env_file::EnvFile;
use crate::XtaskResult;

pub(super) fn ensure(env: &mut EnvFile) -> XtaskResult<()> {
    if let Some(existing) = env.nonempty("DATABASE_URL")? {
        return reconcile_existing_url(env, &existing);
    }

    let user = ensure_value(env, "POSTGRES_USER", "postgres")?;
    let database = ensure_value(env, "POSTGRES_DB", "cheenhub")?;
    let port = ensure_value(env, "POSTGRES_PORT", "5432")?;
    validate_port(&port)?;
    let password = match env.nonempty("POSTGRES_PASSWORD")? {
        Some(value) => value,
        None => {
            let mut bytes = [0_u8; 32];
            UnwrapErr(SysRng).fill_bytes(&mut bytes);
            let generated = URL_SAFE_NO_PAD.encode(bytes);
            env.set("POSTGRES_PASSWORD", &generated)?;
            generated
        }
    };

    let mut url = Url::parse("postgres://127.0.0.1")
        .map_err(|error| format!("cannot prepare database URL: {error}"))?;
    url.set_username(&user)
        .map_err(|()| "invalid POSTGRES_USER for database URL".to_owned())?;
    url.set_password(Some(&password))
        .map_err(|()| "invalid POSTGRES_PASSWORD for database URL".to_owned())?;
    url.set_port(Some(validate_port(&port)?))
        .map_err(|()| "invalid POSTGRES_PORT for database URL".to_owned())?;
    url.set_path(&format!("/{database}"));
    env.set("DATABASE_URL", url.as_str())?;
    Ok(())
}

fn reconcile_existing_url(env: &mut EnvFile, existing: &str) -> XtaskResult<()> {
    let url = Url::parse(existing).map_err(|error| format!("invalid DATABASE_URL: {error}"))?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        return Err("DATABASE_URL must use postgres:// or postgresql://".into());
    }
    if url.host_str().is_none() {
        return Err("DATABASE_URL must contain a host".into());
    }
    let user = percent_decode(url.username(), "DATABASE_URL username")?;
    let password = percent_decode(
        url.password()
            .ok_or("DATABASE_URL must contain a password")?,
        "DATABASE_URL password",
    )?;
    let database = percent_decode(
        url.path().strip_prefix('/').unwrap_or_default(),
        "DATABASE_URL database",
    )?;
    if user.is_empty() || password.is_empty() || database.is_empty() || database.contains('/') {
        return Err("DATABASE_URL must contain a username, password, and database name".into());
    }
    let port = url.port().unwrap_or(5432).to_string();

    // Никаких изменений при конфликте между Compose и DATABASE_URL.
    for (key, expected) in [
        ("POSTGRES_USER", user.as_str()),
        ("POSTGRES_DB", database.as_str()),
        ("POSTGRES_PORT", port.as_str()),
        ("POSTGRES_PASSWORD", password.as_str()),
    ] {
        if let Some(actual) = env.nonempty(key)?
            && actual != expected
        {
            return Err(format!(
                "{key} conflicts with DATABASE_URL; correct the local .env manually"
            ));
        }
    }
    for (key, value) in [
        ("POSTGRES_USER", user.as_str()),
        ("POSTGRES_DB", database.as_str()),
        ("POSTGRES_PORT", port.as_str()),
        ("POSTGRES_PASSWORD", password.as_str()),
    ] {
        if env.nonempty(key)?.is_none() {
            env.set(key, value)?;
        }
    }
    Ok(())
}

fn ensure_value(env: &mut EnvFile, key: &str, default: &str) -> XtaskResult<String> {
    if let Some(value) = env.nonempty(key)? {
        return Ok(value);
    }
    env.set(key, default)?;
    Ok(default.to_owned())
}

fn validate_port(value: &str) -> XtaskResult<u16> {
    let port = value
        .parse::<u16>()
        .map_err(|_| "POSTGRES_PORT must be a valid TCP port".to_owned())?;
    if port == 0 {
        return Err("POSTGRES_PORT must not be zero".into());
    }
    Ok(port)
}

fn percent_decode(input: &str, field: &str) -> XtaskResult<String> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes[offset] == b'%' {
            let pair = bytes
                .get(offset + 1..offset + 3)
                .ok_or_else(|| format!("invalid percent encoding in {field}"))?;
            let hex = |byte: u8| {
                (byte as char)
                    .to_digit(16)
                    .ok_or_else(|| format!("invalid percent encoding in {field}"))
            };
            decoded.push((hex(pair[0])? * 16 + hex(pair[1])?) as u8);
            offset += 3;
        } else {
            decoded.push(bytes[offset]);
            offset += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| format!("invalid UTF-8 in {field}"))
}

#[cfg(test)]
mod tests;
