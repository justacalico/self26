//! SQLite storage layer.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection};

use crate::models::*;

pub type DbRef = Arc<Mutex<Connection>>;

pub fn ts(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

pub fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc.timestamp_opt(0, 0).unwrap())
}

/// Opens (and migrates) the database at `path`, or an in-memory database
/// when `path` is `None`.
pub fn open(path: Option<&Path>) -> Result<DbRef> {
    let conn = match path {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create data dir {}", parent.display()))?;
            }
            Connection::open(p).with_context(|| format!("open database {}", p.display()))?
        }
        None => Connection::open_in_memory().context("open in-memory database")?,
    };
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            api_key TEXT NOT NULL UNIQUE,
            level INTEGER NOT NULL DEFAULT 20,
            avatar_id INTEGER,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            change_seq INTEGER NOT NULL DEFAULT 0,
            uploader_id INTEGER NOT NULL,
            approver_id INTEGER,
            md5 TEXT NOT NULL UNIQUE,
            ext TEXT NOT NULL,
            size INTEGER NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            duration REAL NOT NULL DEFAULT 0,
            has_sample INTEGER NOT NULL DEFAULT 0,
            preview_width INTEGER NOT NULL DEFAULT 0,
            preview_height INTEGER NOT NULL DEFAULT 0,
            sample_width INTEGER NOT NULL DEFAULT 0,
            sample_height INTEGER NOT NULL DEFAULT 0,
            rating TEXT NOT NULL DEFAULT 's',
            sources TEXT NOT NULL DEFAULT '[]',
            description TEXT NOT NULL DEFAULT '',
            parent_id INTEGER,
            is_pending INTEGER NOT NULL DEFAULT 0,
            is_flagged INTEGER NOT NULL DEFAULT 0,
            is_deleted INTEGER NOT NULL DEFAULT 0,
            note_locked INTEGER NOT NULL DEFAULT 0,
            status_locked INTEGER NOT NULL DEFAULT 0,
            rating_locked INTEGER NOT NULL DEFAULT 0,
            locked_tags TEXT NOT NULL DEFAULT '[]'
        );
        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            category INTEGER NOT NULL DEFAULT 0,
            post_count INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS post_tags (
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (post_id, tag_id)
        );
        CREATE TABLE IF NOT EXISTS tag_aliases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            antecedent TEXT NOT NULL,
            consequent TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tag_implications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            antecedent TEXT NOT NULL,
            consequent TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS votes (
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            score INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (post_id, user_id)
        );
        CREATE TABLE IF NOT EXISTS favorites (
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL,
            PRIMARY KEY (post_id, user_id)
        );
        CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            creator_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            body TEXT NOT NULL,
            score INTEGER NOT NULL DEFAULT 0,
            is_hidden INTEGER NOT NULL DEFAULT 0,
            is_sticky INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS comment_votes (
            comment_id INTEGER NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            score INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (comment_id, user_id)
        );
        CREATE TABLE IF NOT EXISTS pools (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            creator_id INTEGER NOT NULL,
            category TEXT NOT NULL DEFAULT 'series',
            is_active INTEGER NOT NULL DEFAULT 1,
            is_deleted INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS pool_posts (
            pool_id INTEGER NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            ord INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (pool_id, post_id)
        );
        CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            creator_id INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            width INTEGER NOT NULL,
            height INTEGER NOT NULL,
            body TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS post_flags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            creator_id INTEGER,
            reason TEXT NOT NULL DEFAULT '',
            is_resolved INTEGER NOT NULL DEFAULT 0,
            is_deletion INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tickets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            creator_id INTEGER,
            qtype TEXT NOT NULL,
            disp_id INTEGER NOT NULL,
            reason TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_post_tags_tag ON post_tags(tag_id);
        CREATE INDEX IF NOT EXISTS idx_posts_created ON posts(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_posts_md5 ON posts(md5);
        CREATE INDEX IF NOT EXISTS idx_fav_user ON favorites(user_id);
        CREATE INDEX IF NOT EXISTS idx_comments_post ON comments(post_id);
        "#,
    )?;
    Ok(())
}

pub fn row_to_post(row: &rusqlite::Row) -> rusqlite::Result<StoredPost> {
    let sources_json: String = row.get("sources")?;
    let locked_json: String = row.get("locked_tags")?;
    Ok(StoredPost {
        id: row.get("id")?,
        created_at: parse_ts(&row.get::<_, String>("created_at")?),
        updated_at: parse_ts(&row.get::<_, String>("updated_at")?),
        change_seq: row.get("change_seq")?,
        uploader_id: row.get("uploader_id")?,
        approver_id: row.get("approver_id")?,
        md5: row.get("md5")?,
        ext: row.get("ext")?,
        size: row.get("size")?,
        width: row.get("width")?,
        height: row.get("height")?,
        duration: row.get("duration")?,
        has_sample: row.get::<_, i64>("has_sample")? != 0,
        preview_width: row.get("preview_width")?,
        preview_height: row.get("preview_height")?,
        sample_width: row.get("sample_width")?,
        sample_height: row.get("sample_height")?,
        rating: Rating::parse(&row.get::<_, String>("rating")?).unwrap_or(Rating::S),
        sources: serde_json::from_str(&sources_json).unwrap_or_default(),
        description: row.get("description")?,
        parent_id: row.get("parent_id")?,
        is_pending: row.get::<_, i64>("is_pending")? != 0,
        is_flagged: row.get::<_, i64>("is_flagged")? != 0,
        is_deleted: row.get::<_, i64>("is_deleted")? != 0,
        note_locked: row.get::<_, i64>("note_locked")? != 0,
        status_locked: row.get::<_, i64>("status_locked")? != 0,
        rating_locked: row.get::<_, i64>("rating_locked")? != 0,
        locked_tags: serde_json::from_str(&locked_json).unwrap_or_default(),
    })
}

pub const POST_COLS: &str = "id, created_at, updated_at, change_seq, uploader_id,
    approver_id, md5, ext, size, width, height, duration, has_sample,
    preview_width, preview_height, sample_width, sample_height, rating, sources,
    description, parent_id, is_pending, is_flagged, is_deleted, note_locked,
    status_locked, rating_locked, locked_tags";

pub fn get_post(conn: &Connection, id: i64) -> Option<StoredPost> {
    let sql = format!("SELECT {POST_COLS} FROM posts WHERE id = ?1");
    conn.query_row(&sql, params![id], row_to_post).ok()
}

pub fn post_tags(conn: &Connection, post_id: i64) -> Vec<(String, i64)> {
    let mut stmt = conn
        .prepare(
            "SELECT t.name, t.category FROM post_tags pt
             JOIN tags t ON t.id = pt.tag_id WHERE pt.post_id = ?1 ORDER BY t.name",
        )
        .expect("prepare post_tags");
    stmt.query_map(params![post_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// Replaces a post's tag set and refreshes tag post_counts.
pub fn set_post_tags(conn: &Connection, post_id: i64, tag_names: &[String]) -> Result<()> {
    conn.execute("DELETE FROM post_tags WHERE post_id = ?1", params![post_id])?;
    for name in tag_names {
        let normalized = name.trim().to_lowercase().replace(' ', "_");
        if normalized.is_empty() {
            continue;
        }
        conn.execute(
            "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![normalized],
        )?;
        let tag_id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![normalized],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO post_tags (post_id, tag_id) VALUES (?1, ?2)",
            params![post_id, tag_id],
        )?;
    }
    refresh_tag_counts(conn)?;
    Ok(())
}

pub fn refresh_tag_counts(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE tags SET post_count = (
            SELECT COUNT(*) FROM post_tags pt
            JOIN posts p ON p.id = pt.post_id
            WHERE pt.tag_id = tags.id AND p.is_deleted = 0
        )",
        [],
    )?;
    Ok(())
}

pub fn user_name(conn: &Connection, id: i64) -> String {
    conn.query_row("SELECT name FROM users WHERE id = ?1", params![id], |r| {
        r.get(0)
    })
    .unwrap_or_else(|_| "unknown".into())
}

pub fn score_parts(conn: &Connection, post_id: i64) -> (i64, i64) {
    conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN score > 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN score < 0 THEN 1 ELSE 0 END), 0)
         FROM votes WHERE post_id = ?1",
        params![post_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap_or((0, 0))
}

pub fn fav_count(conn: &Connection, post_id: i64) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM favorites WHERE post_id = ?1",
        params![post_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn comment_count(conn: &Connection, post_id: i64) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM comments WHERE post_id = ?1 AND is_hidden = 0",
        params![post_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn user_vote(conn: &Connection, post_id: i64, user_id: Option<i64>) -> i64 {
    match user_id {
        Some(uid) => conn
            .query_row(
                "SELECT score FROM votes WHERE post_id = ?1 AND user_id = ?2",
                params![post_id, uid],
                |r| r.get(0),
            )
            .unwrap_or(0),
        None => 0,
    }
}

pub fn is_favorited(conn: &Connection, post_id: i64, user_id: Option<i64>) -> bool {
    match user_id {
        Some(uid) => conn
            .query_row(
                "SELECT 1 FROM favorites WHERE post_id = ?1 AND user_id = ?2",
                params![post_id, uid],
                |_| Ok(()),
            )
            .is_ok(),
        None => false,
    }
}

pub fn post_pools(conn: &Connection, post_id: i64) -> Vec<i64> {
    conn.prepare("SELECT pool_id FROM pool_posts WHERE post_id = ?1")
        .and_then(|mut s| {
            let rows = s.query_map(params![post_id], |r| r.get(0))?;
            Ok(rows.flatten().collect())
        })
        .unwrap_or_default()
}

pub fn post_children(conn: &Connection, post_id: i64) -> Vec<i64> {
    conn.prepare("SELECT id FROM posts WHERE parent_id = ?1 AND is_deleted = 0")
        .and_then(|mut s| {
            let rows = s.query_map(params![post_id], |r| r.get(0))?;
            Ok(rows.flatten().collect())
        })
        .unwrap_or_default()
}
