mod common;

use self26::db;
use self26::models::{Rating, StoredPost};
use self26::api::post_json::{post_json, PostFormat};
use self26::search::{self, Page};

fn page() -> Page {
    Page {
        limit: 75,
        offset: 0,
    }
}

#[test]
fn parse_simple_tag() {
    let q = search::parse("cat", page());
    assert!(q.where_sql.contains("EXISTS"));
    assert!(q.where_sql.contains("t.name = ?1"));
    assert_eq!(q.params.len(), 1);
    assert_eq!(q.order_sql, "posts.id DESC");
}

#[test]
fn parse_negated_and_optional_tags() {
    let q = search::parse("cat -dog ~bird ~fish", page());
    assert!(q.where_sql.contains("NOT EXISTS"));
    assert!(q.where_sql.contains(" OR "));
    assert_eq!(q.params.len(), 4);
}

#[test]
fn parse_tag_normalizes_case() {
    let q = search::parse("Cat", page());
    let conn = db::open(None).unwrap();
    let guard = conn.lock().unwrap();
    guard
        .execute("INSERT INTO tags (name) VALUES ('cat')", [])
        .unwrap();
    let sql = format!(
        "SELECT t.name FROM tags t WHERE t.name = ?1",
    );
    let name: String = guard
        .query_row(&sql, [&q.params[0] as &dyn rusqlite::ToSql], |r| r.get(0))
        .unwrap();
    assert_eq!(name, "cat");
}

#[test]
fn parse_rating_metatag() {
    let q = search::parse("rating:e", page());
    assert!(q.where_sql.contains("posts.rating = ?1"));
    let neg = search::parse("-rating:e", page());
    assert!(neg.where_sql.contains("NOT (posts.rating = ?1)"));
    let invalid = search::parse("rating:bogus", page());
    assert!(!invalid.where_sql.contains("rating"));
}

#[test]
fn parse_id_lists_and_ranges() {
    let q = search::parse("id:3,7..9", page());
    assert!(q.where_sql.contains("posts.id = ?1"));
    assert!(q.where_sql.contains("BETWEEN ?2 AND ?3"));
    assert!(q.where_sql.contains(" OR "));
}

#[test]
fn parse_comparison_metatags() {
    for (term, frag) in [
        ("score:>10", ">"),
        ("score:<=-2", "<="),
        ("width:>=100", ">="),
        ("height:<50", "<"),
        ("fav_count:>3", ">"),
        ("tagcount:2", "="),
    ] {
        let q = search::parse(term, page());
        assert!(q.where_sql.contains(frag), "{term} -> {}", q.where_sql);
        assert_eq!(q.params.len(), 1, "{term}");
    }
}

#[test]
fn parse_status_parent_md5() {
    let q = search::parse("status:deleted", page());
    assert!(q.where_sql.contains("posts.is_deleted = 1"));
    let q = search::parse("parent:none", page());
    assert!(q.where_sql.contains("posts.parent_id IS NULL"));
    let q = search::parse("md5:ABCDEF", page());
    assert!(q.where_sql.contains("posts.md5 = ?1"));
}

#[test]
fn parse_order_metatags() {
    assert!(search::parse("order:score", page())
        .order_sql
        .contains("DESC"));
    assert!(search::parse("order:score_asc", page())
        .order_sql
        .contains("ASC"));
    assert_eq!(search::parse("order:random", page()).order_sql, "RANDOM()");
    assert_eq!(search::parse("order:id_asc", page()).order_sql, "posts.id ASC");
}

#[test]
fn parse_age_direction() {
    let q = search::parse("age:>7d", page());
    assert!(q.where_sql.contains("> ?1"), "{}", q.where_sql);
    let q = search::parse("age:<7d", page());
    assert!(q.where_sql.contains("< ?1"), "{}", q.where_sql);
}

#[test]
fn generated_sql_is_valid_against_schema() {
    let conn = db::open(None).unwrap();
    let guard = conn.lock().unwrap();
    for tags in [
        "cat",
        "-cat dog",
        "rating:e",
        "pool:1",
        "pool:my_pool",
        "fav:admin",
        "favoritedby:2",
        "uploader:someone",
        "type:png",
        "score:>5",
        "fav_count:>=1",
        "comment_count:0",
        "width:>100 height:<200",
        "status:flagged",
        "parent:12",
        "md5:abc",
        "description:*word*",
        "source:*example*",
        "note:hello",
        "tagcount:>2",
        "date:>2020-01-01",
        "age:>7d",
        "id:1,2..4",
        "custom:tag",
    ] {
        let q = search::parse(tags, page());
        let sql = format!(
            "SELECT {} FROM posts WHERE posts.is_deleted = 0 {} ORDER BY {} LIMIT {}",
            db::POST_COLS,
            if q.where_sql.is_empty() {
                String::new()
            } else {
                format!("AND {}", q.where_sql)
            },
            q.order_sql,
            q.limit
        );
        let mut stmt = guard.prepare(&sql).unwrap_or_else(|e| {
            panic!("invalid sql for '{tags}': {e}\n{sql}")
        });
        let refs: Vec<&dyn rusqlite::ToSql> =
            q.params.iter().map(|p| p.as_ref()).collect();
        stmt.query(refs.as_slice()).unwrap_or_else(|e| {
            panic!("query failed for '{tags}': {e}")
        });
    }
}

#[test]
fn page_params_boundaries_and_offsets() {
    let (p, b) = search::page_params(Some("b42"), None);
    assert_eq!(b, Some(('b', 42)));
    assert_eq!(p.limit, 75);
    let (_, b) = search::page_params(Some("a7"), None);
    assert_eq!(b, Some(('a', 7)));
    let (p, b) = search::page_params(Some("3"), Some(10));
    assert_eq!(p.offset, 20);
    assert_eq!(p.limit, 10);
    assert!(b.is_none());
    let (p, _) = search::page_params(None, Some(9999));
    assert_eq!(p.limit, 320);
}

#[test]
fn rating_parse_variants() {
    assert_eq!(Rating::parse("s"), Some(Rating::S));
    assert_eq!(Rating::parse("E"), Some(Rating::E));
    assert_eq!(Rating::parse("questionable"), Some(Rating::Q));
    assert_eq!(Rating::parse("bogus"), None);
}

#[test]
fn post_format_from_params() {
    assert_eq!(
        PostFormat::from_params(None, None),
        PostFormat::Legacy
    );
    assert_eq!(
        PostFormat::from_params(Some("true"), None),
        PostFormat::V2Basic
    );
    assert_eq!(
        PostFormat::from_params(Some("true"), Some("extended")),
        PostFormat::V2Extended
    );
    assert_eq!(
        PostFormat::from_params(Some("1"), Some("thumbnail")),
        PostFormat::V2Thumbnail
    );
    assert_eq!(
        PostFormat::from_params(Some("false"), Some("extended")),
        PostFormat::Legacy
    );
}

fn stored_post() -> StoredPost {
    StoredPost {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        change_seq: 0,
        uploader_id: 1,
        approver_id: Some(1),
        md5: "0123456789abcdef0123456789abcdef".into(),
        ext: "png".into(),
        size: 1234,
        width: 640,
        height: 480,
        duration: 0.0,
        has_sample: false,
        preview_width: 150,
        preview_height: 113,
        sample_width: 640,
        sample_height: 480,
        rating: Rating::S,
        sources: vec!["https://x".into()],
        description: "desc".into(),
        parent_id: None,
        is_pending: false,
        is_flagged: false,
        is_deleted: false,
        note_locked: false,
        status_locked: false,
        rating_locked: false,
        locked_tags: vec![],
    }
}

#[test]
fn post_json_legacy_shape() {
    let db = db::open(None).unwrap();
    let conn = db.lock().unwrap();
    db::set_post_tags(&conn, 1, &["cat".into(), "dog".into()]).unwrap();
    let v = post_json(&conn, &stored_post(), "http://h", None, PostFormat::Legacy);
    assert_eq!(v["id"], 1);
    assert!(v["file"]["url"].as_str().unwrap().starts_with("http://h/data/original/01/"));
    assert!(v["tags"]["general"].as_array().unwrap().len() == 2);
    assert_eq!(v["tag_string"], "cat dog");
    assert!(v["flags"]["deleted"].is_boolean());
    assert!(v["score"]["total"].is_number());
    assert!(v["duration"].is_null());
    assert_eq!(v["is_favorited"], false);
}

#[test]
fn post_json_thumbnail_shape() {
    let db = db::open(None).unwrap();
    let conn = db.lock().unwrap();
    let v = post_json(
        &conn,
        &stored_post(),
        "http://h",
        None,
        PostFormat::V2Thumbnail,
    );
    assert!(v["file_url"].is_string());
    assert_eq!(v["file_ext"], "png");
    assert_eq!(v["score"], 0);
    assert_eq!(v["flags"], "");
    assert!(v["tags"].is_array());
}
