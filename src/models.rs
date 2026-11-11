//! Database row types shared across the API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    S,
    Q,
    E,
}

impl Rating {
    pub fn as_str(self) -> &'static str {
        match self {
            Rating::S => "s",
            Rating::Q => "q",
            Rating::E => "e",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "s" | "safe" => Some(Rating::S),
            "q" | "questionable" => Some(Rating::Q),
            "e" | "explicit" => Some(Rating::E),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoredPost {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub change_seq: i64,
    pub uploader_id: i64,
    pub approver_id: Option<i64>,
    pub md5: String,
    pub ext: String,
    pub size: i64,
    pub width: i64,
    pub height: i64,
    pub duration: f64,
    pub has_sample: bool,
    pub preview_width: i64,
    pub preview_height: i64,
    pub sample_width: i64,
    pub sample_height: i64,
    pub rating: Rating,
    pub sources: Vec<String>,
    pub description: String,
    pub parent_id: Option<i64>,
    pub is_pending: bool,
    pub is_flagged: bool,
    pub is_deleted: bool,
    pub note_locked: bool,
    pub status_locked: bool,
    pub rating_locked: bool,
    pub locked_tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StoredTag {
    pub id: i64,
    pub name: String,
    pub category: i64,
    pub post_count: i64,
}

#[derive(Debug, Clone)]
pub struct StoredUser {
    pub id: i64,
    pub name: String,
    pub password_hash: String,
    pub api_key: String,
    pub level: i64,
    pub created_at: DateTime<Utc>,
    pub avatar_id: Option<i64>,
}

impl StoredUser {
    pub const LEVEL_MEMBER: i64 = 20;
    pub const LEVEL_ADMIN: i64 = 50;

    pub fn is_admin(&self) -> bool {
        self.level >= Self::LEVEL_ADMIN
    }
}

#[derive(Debug, Clone)]
pub struct StoredComment {
    pub id: i64,
    pub post_id: i64,
    pub creator_id: i64,
    pub body: String,
    pub score: i64,
    pub is_hidden: bool,
    pub is_sticky: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoredPool {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub creator_id: i64,
    pub category: String,
    pub is_active: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub post_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct StoredNote {
    pub id: i64,
    pub post_id: i64,
    pub creator_id: i64,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub body: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoredFlag {
    pub id: i64,
    pub post_id: i64,
    pub creator_id: Option<i64>,
    pub reason: String,
    pub is_resolved: bool,
    pub is_deletion: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoredTicket {
    pub id: i64,
    pub creator_id: Option<i64>,
    pub qtype: String,
    pub disp_id: i64,
    pub reason: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// e621 tag categories.
pub mod tag_category {
    pub const GENERAL: i64 = 0;
    pub const ARTIST: i64 = 1;
    pub const CONTRIBUTOR: i64 = 2;
    pub const COPYRIGHT: i64 = 3;
    pub const CHARACTER: i64 = 4;
    pub const SPECIES: i64 = 5;
    pub const INVALID: i64 = 6;
    pub const META: i64 = 7;
    pub const LORE: i64 = 8;

    pub fn name(category: i64) -> &'static str {
        match category {
            ARTIST => "artist",
            CONTRIBUTOR => "contributor",
            COPYRIGHT => "copyright",
            CHARACTER => "character",
            SPECIES => "species",
            INVALID => "invalid",
            META => "meta",
            LORE => "lore",
            _ => "general",
        }
    }
}
