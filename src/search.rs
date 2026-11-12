//! e621-style search syntax → SQL. Supports plain tags, `-` negation,
//! `~` optional tags, wildcard `*` tags, and the common metatags
//! (`rating:`, `id:`, `score:`, `fav_count:`, `order:`, `status:`, `parent:`,
//! `md5:`, `uploader:`/`user:`, `type:`, `width:`, `height:`, `fav:`,
//! `pool:`, `description:`, `source:`, `date:`).

use rusqlite::ToSql;

#[derive(Default)]
pub struct SearchQuery {
    pub where_sql: String,
    pub params: Vec<Box<dyn ToSql>>,
    pub order_sql: String,
    pub limit: i64,
}

fn push_param(params: &mut Vec<Box<dyn ToSql>>, v: impl ToSql + 'static) -> String {
    params.push(Box::new(v));
    format!("?{}", params.len())
}

fn parse_cmp(term: &str) -> (&str, &str) {
    for op in [">=", "<=", ">", "<"] {
        if let Some(rest) = term.strip_prefix(op) {
            return (op, rest);
        }
    }
    ("=", term)
}

/// Parse `search[id]`-style `,`-separated id lists and ranges `a..b`.
fn id_condition(field: &str, raw: &str, params: &mut Vec<Box<dyn ToSql>>) -> Option<String> {
    let mut parts = Vec::new();
    for piece in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some((lo, hi)) = piece.split_once("..") {
            if let (Ok(a), Ok(b)) = (lo.parse::<i64>(), hi.parse::<i64>()) {
                let p1 = push_param(params, a);
                let p2 = push_param(params, b);
                parts.push(format!("{field} BETWEEN {p1} AND {p2}"));
            }
        } else if let Ok(v) = piece.parse::<i64>() {
            let p = push_param(params, v);
            parts.push(format!("{field} = {p}"));
        }
    }
    match parts.len() {
        0 => None,
        1 => Some(parts.pop().unwrap()),
        _ => Some(format!("({})", parts.join(" OR "))),
    }
}

fn tag_subquery(tag: &str, params: &mut Vec<Box<dyn ToSql>>, negate: bool) -> String {
    let tag = tag.to_lowercase().replace(' ', "_");
    let clause = if tag.contains('*') {
        let p = push_param(params, tag.replace('*', "%"));
        format!(
            "EXISTS (SELECT 1 FROM post_tags pt JOIN tags t ON t.id = pt.tag_id
                 WHERE pt.post_id = posts.id AND t.name LIKE {p} ESCAPE '\\')"
        )
    } else {
        let p = push_param(params, tag.to_string());
        format!(
            "EXISTS (SELECT 1 FROM post_tags pt JOIN tags t ON t.id = pt.tag_id
                 WHERE pt.post_id = posts.id AND t.name = {p})"
        )
    };
    if negate {
        format!("NOT {clause}")
    } else {
        clause
    }
}

/// Parse a tag string into a query. `default_limit` applies when the caller
/// passes no `limit` param.
pub fn parse(tags: &str, page: Page) -> SearchQuery {
    let mut q = SearchQuery {
        order_sql: "posts.id DESC".into(),
        limit: page.limit,
        ..Default::default()
    };
    let mut conds: Vec<String> = Vec::new();
    let mut optionals: Vec<String> = Vec::new();
    let mut random_order = false;

    for raw in tags.split_whitespace() {
        let (neg, term) = match raw.strip_prefix('-') {
            Some(t) => (true, t),
            None => (false, raw),
        };
        let opt = term.starts_with('~');
        let term = term.trim_start_matches('~');
        if term.is_empty() {
            continue;
        }

        let cond = if let Some((key, value)) = term.split_once(':') {
            meta_condition(
                key,
                value,
                &mut q.params,
                &mut q.order_sql,
                &mut random_order,
                neg,
            )
        } else if opt {
            optionals.push(tag_subquery(term, &mut q.params, false));
            continue;
        } else {
            Some(tag_subquery(term, &mut q.params, neg))
        };
        if let Some(c) = cond {
            conds.push(c);
        }
    }
    if !optionals.is_empty() {
        conds.push(format!("({})", optionals.join(" OR ")));
    }

    if random_order {
        q.order_sql = "RANDOM()".into();
    }
    q.where_sql = conds.join(" AND ");
    q
}

#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub limit: i64,
    pub offset: i64,
}

/// Resolve e621 `page`/`limit` params (page may be a number or `b<ID>`/`a<ID>`).
pub fn page_params(page: Option<&str>, limit: Option<i64>) -> (Page, Option<(char, i64)>) {
    let limit = limit.unwrap_or(75).clamp(0, 320);
    let mut offset = 0i64;
    let mut boundary = None;
    if let Some(p) = page {
        if let Some(id) = p.strip_prefix('b').and_then(|s| s.parse::<i64>().ok()) {
            boundary = Some(('b', id));
        } else if let Some(id) = p.strip_prefix('a').and_then(|s| s.parse::<i64>().ok()) {
            boundary = Some(('a', id));
        } else if let Ok(n) = p.parse::<i64>() {
            offset = (n - 1).max(0).saturating_mul(limit);
        }
    }
    (Page { limit, offset }, boundary)
}

fn meta_condition(
    key: &str,
    value: &str,
    params: &mut Vec<Box<dyn ToSql>>,
    order_sql: &mut String,
    random_order: &mut bool,
    neg: bool,
) -> Option<String> {
    let cond = match key {
        "rating" => {
            let r = crate::models::Rating::parse(value)?;
            let p = push_param(params, r.as_str());
            format!("posts.rating = {p}")
        }
        "id" => id_condition("posts.id", value, params)?,
        "score" | "fav_count" | "width" | "height" | "change_seq" | "comment_count" => {
            let (op, v) = parse_cmp(value);
            let n: i64 = v.parse().ok()?;
            let field = match key {
                "fav_count" => {
                    "(SELECT COUNT(*) FROM favorites f WHERE f.post_id = posts.id)".to_string()
                }
                "score" => {
                    "(SELECT COALESCE(SUM(score),0) FROM votes v WHERE v.post_id = posts.id)"
                        .to_string()
                }
                "comment_count" => {
                    "(SELECT COUNT(*) FROM comments c WHERE c.post_id = posts.id AND c.is_hidden = 0)"
                        .to_string()
                }
                _ => format!("posts.{key}"),
            };
            let p = push_param(params, n);
            format!("{field} {op} {p}")
        }
        "status" => match value {
            "deleted" => "posts.is_deleted = 1".into(),
            "pending" => "posts.is_pending = 1".into(),
            "flagged" => "posts.is_flagged = 1".into(),
            "active" => "posts.is_deleted = 0 AND posts.is_pending = 0".into(),
            _ => "posts.is_deleted = 0".into(),
        },
        "parent" => match value {
            "none" => "posts.parent_id IS NULL".into(),
            "any" => "posts.parent_id IS NOT NULL".into(),
            _ => {
                let n: i64 = value.parse().ok()?;
                let p = push_param(params, n);
                format!("posts.parent_id = {p}")
            }
        },
        "md5" => {
            let p = push_param(params, value.to_lowercase());
            format!("posts.md5 = {p}")
        }
        "uploader" | "user" | "approver" => {
            let field = if key == "approver" {
                "posts.approver_id"
            } else {
                "posts.uploader_id"
            };
            if let Ok(n) = value.parse::<i64>() {
                let p = push_param(params, n);
                format!("{field} = {p}")
            } else {
                let p = push_param(params, value.to_string());
                format!("{field} = (SELECT id FROM users WHERE name = {p} COLLATE NOCASE)")
            }
        }
        "type" | "filetype" | "ext" => {
            let p = push_param(params, value.to_lowercase());
            format!("posts.ext = {p}")
        }
        "pool" => {
            let p = push_param(params, value.to_string());
            format!(
                "posts.id IN (SELECT post_id FROM pool_posts pp
                     JOIN pools pl ON pl.id = pp.pool_id
                     WHERE pl.name = {p} OR CAST(pl.id AS TEXT) = {p})"
            )
        }
        "fav" | "favoritedby" | "favorited_by" => {
            let p = push_param(params, value.to_string());
            format!(
                "posts.id IN (SELECT post_id FROM favorites f
                     JOIN users u ON u.id = f.user_id
                     WHERE u.name = {p} COLLATE NOCASE OR CAST(u.id AS TEXT) = {p})"
            )
        }
        "description" | "desc" => {
            let p = push_param(params, format!("%{}%", value.replace('*', "%")));
            format!("posts.description LIKE {p} ESCAPE '\\'")
        }
        "source" => {
            let p = push_param(params, format!("%{}%", value.replace('*', "%")));
            format!("posts.sources LIKE {p} ESCAPE '\\'")
        }
        "note" | "notes" => {
            let p = push_param(params, format!("%{}%", value.replace('*', "%")));
            format!(
                "posts.id IN (SELECT post_id FROM notes n
                     WHERE n.post_id = posts.id AND n.is_active = 1
                     AND n.body LIKE {p} ESCAPE '\\')"
            )
        }
        "order" => {
            *order_sql = match value {
                "id_asc" | "id" => "posts.id ASC".into(),
                "score" | "score_desc" => {
                    "(SELECT COALESCE(SUM(score),0) FROM votes v WHERE v.post_id = posts.id) DESC, posts.id DESC"
                        .into()
                }
                "score_asc" => {
                    "(SELECT COALESCE(SUM(score),0) FROM votes v WHERE v.post_id = posts.id) ASC, posts.id DESC"
                        .into()
                }
                "favcount" | "fav_count" => {
                    "(SELECT COUNT(*) FROM favorites f WHERE f.post_id = posts.id) DESC, posts.id DESC"
                        .into()
                }
                "updated" | "updated_desc" => "posts.updated_at DESC, posts.id DESC".into(),
                "updated_asc" => "posts.updated_at ASC, posts.id ASC".into(),
                "rank" | "hot" => {
                    "(SELECT COALESCE(SUM(score),0) FROM votes v WHERE v.post_id = posts.id)
                     / (1.0 + (julianday('now') - julianday(posts.created_at))) DESC"
                        .into()
                }
                "random" => {
                    *random_order = true;
                    "posts.id DESC".into()
                }
                "change_seq" | "change_seq_desc" => "posts.change_seq DESC".into(),
                _ => "posts.id DESC".into(),
            };
            return None;
        }
        "date" | "age" => {
            let (op, v) = parse_cmp(value);
            if key == "date" {
                let p = push_param(params, v.to_string());
                format!("date(posts.created_at) {op} date({p})")
            } else {
                let secs = parse_age(v).unwrap_or(0);
                let p = push_param(params, secs);
                format!("(julianday('now') - julianday(posts.created_at)) * 86400.0 {op} {p}")
            }
        }
        "tagcount" => {
            let (op, v) = parse_cmp(value);
            let n: i64 = v.parse().ok()?;
            let p = push_param(params, n);
            format!("(SELECT COUNT(*) FROM post_tags pt WHERE pt.post_id = posts.id) {op} {p}")
        }
        "limit" => return None,
        _ => {
            // Unknown "foo:bar" — e621 treats it as a literal tag name.
            return Some(tag_subquery(term_rejoin(key, value).as_str(), params, neg));
        }
    };
    Some(if neg { format!("NOT ({cond})") } else { cond })
}

fn term_rejoin(key: &str, value: &str) -> String {
    format!("{key}:{value}")
}

/// Parse an `age:` value like `1d`, `2w`, `3h`, `30m`, `1y` into seconds.
fn parse_age(v: &str) -> Option<i64> {
    let split = v
        .rfind(|c: char| c.is_ascii_digit())
        .map(|i| i + 1)
        .unwrap_or(v.len());
    let (num, unit) = v.split_at(split);
    let n: i64 = num.parse().ok()?;
    Some(match unit {
        "s" => n,
        "mi" | "m" => n * 60,
        "h" => n * 3600,
        "d" => n * 86400,
        "w" => n * 604800,
        "mo" => n * 2592000,
        "y" => n * 31536000,
        _ => return None,
    })
}
