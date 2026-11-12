//! Password hashing, API keys, sessions and request auth.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use base64::Engine;
use chrono::Utc;
use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::{params, Connection};
use subtle::ConstantTimeEq;

use crate::db;
use crate::models::StoredUser;
use crate::AppState;

pub fn hash_password(password: &str) -> Option<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .ok()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn new_api_key() -> String {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    hex::encode(buf)
}

pub fn new_session_token() -> String {
    let mut buf = [0u8; 48];
    OsRng.fill_bytes(&mut buf);
    hex::encode(buf)
}

pub fn create_user(
    conn: &Connection,
    name: &str,
    password: &str,
    level: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO users (name, password_hash, api_key, level, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            name,
            hash_password(password).ok_or(rusqlite::Error::InvalidQuery)?,
            new_api_key(),
            level,
            db::ts(Utc::now())
        ],
    )?;
    conn.query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
}

pub fn get_user(conn: &Connection, id: i64) -> Option<StoredUser> {
    conn.query_row(
        "SELECT id, name, password_hash, api_key, level, created_at, avatar_id
         FROM users WHERE id = ?1",
        params![id],
        |r| {
            Ok(StoredUser {
                id: r.get(0)?,
                name: r.get(1)?,
                password_hash: r.get(2)?,
                api_key: r.get(3)?,
                level: r.get(4)?,
                created_at: db::parse_ts(&r.get::<_, String>(5)?),
                avatar_id: r.get(6)?,
            })
        },
    )
    .ok()
}

pub fn user_by_name(conn: &Connection, name: &str) -> Option<StoredUser> {
    conn.query_row(
        "SELECT id FROM users WHERE name = ?1 COLLATE NOCASE",
        params![name],
        |r| r.get::<_, i64>(0),
    )
    .ok()
    .and_then(|id| get_user(conn, id))
}

pub fn user_by_api_key(conn: &Connection, key: &str) -> Option<StoredUser> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM users WHERE api_key = ?1",
            params![key],
            |r| r.get(0),
        )
        .ok();
    let id = id?;
    let user = get_user(conn, id)?;
    // Constant-time compare on top of the lookup to avoid timing leaks.
    if user.api_key.as_bytes().ct_eq(key.as_bytes()).into() {
        Some(user)
    } else {
        None
    }
}

pub fn create_session(conn: &Connection, user_id: i64) -> rusqlite::Result<String> {
    let token = new_session_token();
    conn.execute(
        "INSERT INTO sessions (token, user_id, created_at) VALUES (?1, ?2, ?3)",
        params![token, user_id, db::ts(Utc::now())],
    )?;
    Ok(token)
}

pub fn user_by_session(conn: &Connection, token: &str) -> Option<StoredUser> {
    let cutoff = db::ts(Utc::now() - chrono::Duration::days(30));
    let _ = conn.execute(
        "DELETE FROM sessions WHERE created_at <= ?1",
        params![cutoff],
    );
    let id: i64 = conn
        .query_row(
            "SELECT user_id FROM sessions WHERE token = ?1 AND created_at > ?2",
            params![token, cutoff],
            |r| r.get(0),
        )
        .ok()?;
    get_user(conn, id)
}

/// The authenticated user for a request, if any. `local` mode (loopback
/// dev server) authenticates every request as the first admin.
#[derive(Clone)]
pub struct ApiAuth(pub Option<StoredUser>);

impl ApiAuth {
    pub fn user(&self) -> Option<&StoredUser> {
        self.0.as_ref()
    }

    pub fn id(&self) -> Option<i64> {
        self.0.as_ref().map(|u| u.id)
    }
}

fn cookie_value<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .map(str::trim)
        .find_map(|c| c.strip_prefix(&format!("{name}=")))
}

#[axum::async_trait]
impl FromRequestParts<AppState> for ApiAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(_) => return Ok(ApiAuth(None)),
        };

        if state.local_mode {
            if let Some(admin) = first_admin(&conn) {
                return Ok(ApiAuth(Some(admin)));
            }
        }

        // HTTP basic auth: login:api_key (e621 style).
        if let Some(value) = parts.headers.get(axum::http::header::AUTHORIZATION) {
            if let Ok(v) = value.to_str() {
                if let Some(b64) = v.strip_prefix("Basic ") {
                    if let Ok(decoded) =
                        base64::engine::general_purpose::STANDARD.decode(b64.trim())
                    {
                        if let Ok(creds) = String::from_utf8(decoded) {
                            if let Some((login, key)) = creds.split_once(':') {
                                if let Some(user) = user_by_name(&conn, login) {
                                    if user.api_key.as_bytes().ct_eq(key.as_bytes()).into() {
                                        return Ok(ApiAuth(Some(user)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ?login=&api_key= query auth (e621 clients use this on uploads).
        let query: Vec<(String, String)> =
            serde_urlencoded_light(parts.uri.query().unwrap_or_default());
        let login = query.iter().find(|(k, _)| k == "login").map(|(_, v)| v);
        let key = query.iter().find(|(k, _)| k == "api_key").map(|(_, v)| v);
        if let (Some(login), Some(key)) = (login, key) {
            if let Some(user) = user_by_name(&conn, login) {
                if user.api_key.as_bytes().ct_eq(key.as_bytes()).into() {
                    return Ok(ApiAuth(Some(user)));
                }
            }
        }

        // Session cookie (web UI). Accepts the danbooru cookie name too.
        for cookie in ["self26_session", "_danbooru_session"] {
            if let Some(token) = cookie_value(&parts.headers, cookie) {
                if let Some(user) = user_by_session(&conn, token) {
                    return Ok(ApiAuth(Some(user)));
                }
            }
        }

        Ok(ApiAuth(None))
    }
}

fn first_admin(conn: &Connection) -> Option<StoredUser> {
    let id: i64 = conn
        .query_row(
            "SELECT id FROM users ORDER BY level DESC, id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok()?;
    get_user(conn, id)
}

/// Tiny query-string parser (no extra dep): `a=1&b=x%20y`.
fn serde_urlencoded_light(q: &str) -> Vec<(String, String)> {
    q.split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (urldecode(k), urldecode(v))
        })
        .collect()
}

fn urldecode(s: &str) -> String {
    percent_encoding::percent_decode_str(&s.replace('+', " "))
        .decode_utf8()
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| s.to_string())
}
