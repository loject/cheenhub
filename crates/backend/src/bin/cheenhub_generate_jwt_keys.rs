#![warn(missing_docs)]
//! Генерирует пару ключей Ed25519 для настройки JWT CheenHub.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

const PRIVATE_KEY_ENV: &str = "JWT_ED25519_PRIVATE_KEY_BASE64";
const BACKEND_KEY_ID_ENV: &str = "JWT_KEY_ID";
const CLIENT_KEY_ID_ENV: &str = "CHEENHUB_JWT_KEY_ID";
const CLIENT_PUBLIC_KEY_ENV: &str = "CHEENHUB_JWT_PUBLIC_KEY_BASE64";
const DEFAULT_LOCAL_KEY_ID: &str = "dev-ed25519-1";

/// Печатает переменные окружения JWT-ключей в формате, пригодном для shell.
fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => {
            print_generated_keys();
            Ok(())
        }
        Some("--ensure-local-env") => {
            let env_path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(".env"));
            let key_id = parse_key_id_arg(args)?;
            ensure_local_env(&env_path, &key_id)
        }
        Some(argument) => bail!("unsupported argument {argument}"),
    }
}

fn print_generated_keys() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let private_key = STANDARD.encode(signing_key.to_bytes());
    let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());

    println!("JWT_ED25519_PRIVATE_KEY_BASE64={private_key}");
    println!("CHEENHUB_JWT_PUBLIC_KEY_BASE64={public_key}");
}

fn parse_key_id_arg(mut args: impl Iterator<Item = String>) -> anyhow::Result<String> {
    let mut key_id = DEFAULT_LOCAL_KEY_ID.to_owned();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--key-id" => {
                key_id = args.next().context("--key-id requires a non-empty value")?;
                if key_id.trim().is_empty() {
                    return Err(anyhow!("--key-id requires a non-empty value"));
                }
            }
            _ => bail!("unsupported argument {argument}"),
        }
    }

    Ok(key_id)
}

fn ensure_local_env(env_path: &Path, default_key_id: &str) -> anyhow::Result<()> {
    let content = fs::read_to_string(env_path)
        .with_context(|| format!("failed to read local env file {}", env_path.display()))?;
    let private_key = env_value(&content, PRIVATE_KEY_ENV)
        .and_then(|value| signing_key_from_base64(&value))
        .unwrap_or_else(|| SigningKey::generate(&mut OsRng));
    let private_key_base64 = STANDARD.encode(private_key.to_bytes());
    let public_key_base64 = STANDARD.encode(private_key.verifying_key().to_bytes());
    let key_id = env_value(&content, BACKEND_KEY_ID_ENV)
        .or_else(|| env_value(&content, CLIENT_KEY_ID_ENV))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_key_id.to_owned());

    let mut updated = content.clone();
    updated = upsert_env_value(&updated, PRIVATE_KEY_ENV, &private_key_base64);
    updated = upsert_env_value(&updated, BACKEND_KEY_ID_ENV, &key_id);
    updated = upsert_env_value(&updated, CLIENT_KEY_ID_ENV, &key_id);
    updated = upsert_env_value(&updated, CLIENT_PUBLIC_KEY_ENV, &public_key_base64);

    if updated == content {
        println!("Local JWT env values are valid in {}.", env_path.display());
        return Ok(());
    }

    fs::write(env_path, updated)
        .with_context(|| format!("failed to write local env file {}", env_path.display()))?;
    println!("Updated local JWT env values in {}.", env_path.display());
    Ok(())
}

fn env_value(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            return None;
        }
        let line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let (line_key, value) = line.split_once('=')?;
        if line_key.trim() != key {
            return None;
        }

        Some(unquote_env_value(value.trim()).to_owned())
    })
}

fn unquote_env_value(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }

    value
}

fn signing_key_from_base64(value: &str) -> Option<SigningKey> {
    let bytes = STANDARD.decode(value.trim()).ok()?;
    let seed: [u8; 32] = bytes.try_into().ok()?;
    Some(SigningKey::from_bytes(&seed))
}

fn upsert_env_value(content: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if env_line_key(line).as_deref() == Some(key) {
            lines.push(format!("{key}={value}"));
            found = true;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !found {
        lines.push(format!("{key}={value}"));
    }

    let mut updated = lines.join("\n");
    if content.ends_with('\n') {
        updated.push('\n');
    }
    updated
}

fn env_line_key(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return None;
    }
    let line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (key, _) = line.split_once('=')?;
    Some(key.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_replaces_existing_value() {
        let content = "JWT_KEY_ID=old\nOTHER=value\n";

        let updated = upsert_env_value(content, "JWT_KEY_ID", "new");

        assert_eq!(updated, "JWT_KEY_ID=new\nOTHER=value\n");
    }

    #[test]
    fn upsert_appends_missing_value() {
        let content = "OTHER=value\n";

        let updated = upsert_env_value(content, "JWT_KEY_ID", "new");

        assert_eq!(updated, "OTHER=value\nJWT_KEY_ID=new\n");
    }

    #[test]
    fn reads_exported_and_quoted_values() {
        let content = "export JWT_KEY_ID=\"quoted\"\n";

        assert_eq!(env_value(content, "JWT_KEY_ID").as_deref(), Some("quoted"));
    }
}
