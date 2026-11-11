mod common;

use common::*;
use serde_json::Value;

async fn list_ids(app: &TestApp, query: &[(&str, &str)]) -> Vec<i64> {
    let res = app
        .get("/posts.json")
        .query(query)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    body["posts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn upload_requires_auth() {
    let app = spawn().await;
    let res = upload(&app, "", "", png_bytes(8, 8, 1), "a.png", &[]).await;
    assert_eq!(res.status(), 401);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn upload_stores_post_and_files() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let png = png_bytes(200, 100, 7);
    let res = upload(
        &app,
        "admin",
        &key,
        png.clone(),
        "pic.png",
        &[
            ("post[tag_string]", "cat cute"),
            ("post[rating]", "e"),
            ("post[source]", "https://example.com/a"),
            ("post[description]", "hello world"),
        ],
    )
    .await;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["success"], true);
    let id = body["post_id"].as_i64().unwrap();

    let post = &body["post"];
    assert_eq!(post["id"], id);
    assert_eq!(post["rating"], "e");
    assert_eq!(post["description"], "hello world");
    assert_eq!(post["file"]["width"], 200);
    assert_eq!(post["file"]["height"], 100);
    assert_eq!(post["file"]["ext"], "png");
    assert_eq!(post["file"]["size"], png.len() as i64);
    assert_eq!(post["file"]["md5"].as_str().unwrap().len(), 32);
    assert!(post["file"]["url"]
        .as_str()
        .unwrap()
        .contains("/data/original/"));
    assert!(post["tag_string"].as_str().unwrap().contains("cat"));
    assert_eq!(post["sources"][0], "https://example.com/a");
    assert_eq!(post["flags"]["deleted"], false);
    assert_eq!(post["flags"]["pending"], false);
    assert_eq!(post["score"]["total"], 0);
}

#[tokio::test]
async fn upload_via_uploads_endpoint_and_jpeg() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let form = reqwest::multipart::Form::new()
        .part(
            "upload[file]",
            reqwest::multipart::Part::bytes(jpeg_bytes(50, 40, 3)).file_name("pic.jpg"),
        )
        .text("upload[tag_string]", "photo");
    let res = app
        .authed(reqwest::Method::POST, "/uploads.json", "admin", &key)
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["file"]["ext"], "jpg");
}

#[tokio::test]
async fn duplicate_md5_rejected() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let bytes = png_bytes(16, 16, 9);
    let res = upload(&app, "admin", &key, bytes.clone(), "a.png", &[]).await;
    assert_eq!(res.status(), 201);
    let res = upload(&app, "admin", &key, bytes, "a.png", &[]).await;
    assert_eq!(res.status(), 422);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["error"], "duplicate");
}

#[tokio::test]
async fn list_legacy_wraps_posts() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let id = seed_post(&app, "admin", &key, 1, "cat").await;
    let res = app.get("/posts.json").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["id"], id);
    assert_eq!(posts[0]["tags"]["general"][0], "cat");
    assert!(posts[0]["relationships"]["has_children"].is_boolean());
    assert!(posts[0]["pools"].is_array());
}

#[tokio::test]
async fn show_post_legacy_and_missing() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let id = seed_post(&app, "admin", &key, 2, "dog").await;

    let res = app.get(&format!("/posts/{id}.json")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["id"], id);
    assert_eq!(body["post"]["tag_string"], "dog");

    let res = app.get("/posts/99999.json").send().await.unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn v2_basic_shape() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let id = seed_post(&app, "admin", &key, 3, "bird flying").await;

    let res = app
        .get("/posts.json")
        .query(&[("v2", "true"), ("mode", "basic")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let posts = body.as_array().unwrap();
    assert_eq!(posts.len(), 1);
    let p = &posts[0];
    assert_eq!(p["id"], id);
    assert!(p["files"]["original"]["url"]
        .as_str()
        .unwrap()
        .contains("/data/original/"));
    assert!(p["files"]["preview"]["jpg"]
        .as_str()
        .unwrap()
        .contains("/data/preview/"));
    assert_eq!(p["files"]["meta"]["ext"], "png");
    assert!(p["tags"].is_array());
    assert!(p["tags"].as_array().unwrap().contains(&Value::from("bird")));
    assert_eq!(p["stats"]["score"]["total"], 0);
    assert_eq!(p["stats"]["fav_count"], 0);
    assert_eq!(p["flags"]["deleted"], false);
    assert_eq!(p["has"]["sample"], false);
    assert_eq!(p["has"]["notes"], false);
    assert!(p["relationships"]["parent_id"].is_null());
    assert_eq!(p["uploader_name"], "admin");
}

#[tokio::test]
async fn v2_extended_categorizes_tags() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let id = seed_post(&app, "admin", &key, 4, "wolf").await;

    let res = app
        .get(&format!("/posts/{id}.json"))
        .query(&[("v2", "true"), ("mode", "extended")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["id"], id);
    let tags = body["tags"].as_object().unwrap();
    assert_eq!(tags["general"][0], "wolf");
    for cat in [
        "artist",
        "character",
        "copyright",
        "species",
        "meta",
        "invalid",
        "lore",
        "contributor",
    ] {
        assert!(tags.contains_key(cat), "missing category {cat}");
    }
    assert_eq!(body["files"]["meta"]["md5"].as_str().unwrap().len(), 32);
    assert!(body["stats"]["vote"].is_number());
    assert_eq!(body["stats"]["is_favorited"], false);
}

#[tokio::test]
async fn v2_thumbnail_flat_shape() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    seed_post(&app, "admin", &key, 5, "fox").await;

    let res = app
        .get("/posts.json")
        .query(&[("v2", "true"), ("mode", "thumbnail")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let p = &body.as_array().unwrap()[0];
    assert!(p["file_url"].as_str().unwrap().contains("/data/original/"));
    assert!(p["preview_url"].as_str().unwrap().contains("/data/preview/"));
    assert_eq!(p["file_ext"], "png");
    assert_eq!(p["md5"].as_str().unwrap().len(), 32);
    assert_eq!(p["score"], 0);
    assert_eq!(p["flags"], "");
    assert!(p["tags"].as_array().unwrap().contains(&Value::from("fox")));
    assert_eq!(p["uploader"], "admin");
    assert_eq!(p["rating"], "s");
}

#[tokio::test]
async fn update_post_edits_tags_rating_description() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let id = seed_post(&app, "admin", &key, 6, "old_tag").await;

    let res = app
        .authed(reqwest::Method::PATCH, &format!("/posts/{id}.json"), "admin", &key)
        .form(&[
            ("post[tag_string]", "new_tag another"),
            ("post[rating]", "q"),
            ("post[description]", "edited"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["rating"], "q");
    assert_eq!(body["post"]["description"], "edited");
    let tag_string = body["post"]["tag_string"].as_str().unwrap();
    assert!(tag_string.contains("new_tag"));
    assert!(!tag_string.contains("old_tag"));
}

#[tokio::test]
async fn non_owner_cannot_edit_or_delete() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "member", &mkey, 7, "x").await;

    let res = app
        .authed(reqwest::Method::PATCH, &format!("/posts/{id}.json"), "admin", &akey)
        .form(&[("post[description]", "admin edit")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let (_, key2) = add_user(&app, "other", "otherpw", 20);
    let res = app
        .authed(reqwest::Method::PATCH, &format!("/posts/{id}.json"), "other", &key2)
        .form(&[("post[description]", "nope")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
    let res = app
        .authed(reqwest::Method::DELETE, &format!("/posts/{id}.json"), "other", &key2)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn delete_post_removes_from_listing() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let id = seed_post(&app, "admin", &key, 8, "gone").await;

    let res = app
        .authed(reqwest::Method::DELETE, &format!("/posts/{id}.json"), "admin", &key)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    assert!(list_ids(&app, &[]).await.is_empty());

    let res = app.get(&format!("/posts/{id}.json")).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["flags"]["deleted"], true);
}

#[tokio::test]
async fn search_filters() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let cat = seed_post(&app, "admin", &key, 10, "cat cute").await;
    let dog = seed_post(&app, "admin", &key, 11, "dog").await;
    let res = app
        .authed(reqwest::Method::PATCH, &format!("/posts/{dog}.json"), "admin", &key)
        .form(&[("post[rating]", "e")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    assert_eq!(list_ids(&app, &[("tags", "cat")]).await, vec![cat]);
    assert_eq!(list_ids(&app, &[("tags", "-cat")]).await, vec![dog]);
    assert_eq!(list_ids(&app, &[("tags", "rating:e")]).await, vec![dog]);
    assert_eq!(list_ids(&app, &[("tags", "-rating:e")]).await, vec![cat]);
    assert_eq!(list_ids(&app, &[("tags", "type:png")]).await.len(), 2);
    assert_eq!(list_ids(&app, &[("tags", "type:jpg")]).await.len(), 0);
    assert_eq!(
        list_ids(&app, &[("tags", &format!("id:{cat}")])).await,
        vec![cat]
    );
    assert_eq!(
        list_ids(&app, &[("tags", &format!("id:{cat},{dog}")])).await.len(),
        2
    );
    assert_eq!(
        list_ids(&app, &[("tags", "uploader:admin")]).await.len(),
        2
    );
    assert_eq!(
        list_ids(&app, &[("tags", "uploader:nobody")]).await.len(),
        0
    );
    assert_eq!(list_ids(&app, &[("tags", "ca*")]).await, vec![cat]);
    assert_eq!(
        list_ids(&app, &[("tags", "~cat ~dog")]).await.len(),
        2
    );
    assert_eq!(
        list_ids(&app, &[("tags", "status:active")]).await.len(),
        2
    );
    assert_eq!(
        list_ids(&app, &[("tags", "parent:none")]).await.len(),
        2
    );
}

#[tokio::test]
async fn search_order_and_pagination() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let p1 = seed_post(&app, "admin", &key, 20, "a").await;
    let p2 = seed_post(&app, "admin", &key, 21, "b").await;
    let p3 = seed_post(&app, "admin", &key, 22, "c").await;

    assert_eq!(
        list_ids(&app, &[("tags", "order:id_asc")]).await,
        vec![p1, p2, p3]
    );
    assert_eq!(
        list_ids(&app, &[("page", &format!("b{p3}"))]).await,
        vec![p2, p1]
    );
    assert_eq!(
        list_ids(&app, &[("limit", "1"), ("page", "2")]).await,
        vec![p2]
    );
}

#[tokio::test]
async fn post_counts_and_random_and_popular() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    seed_post(&app, "admin", &key, 30, "cat").await;
    seed_post(&app, "admin", &key, 31, "dog").await;

    let res = app.get("/post_counts.json").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["counts"]["posts"], 2);

    let res = app
        .get("/post_counts.json")
        .query(&[("tags", "cat")])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["counts"]["posts"], 1);

    let res = app.get("/posts/random.json").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert!(body["post"]["id"].is_number());

    for path in ["/popular.json", "/explore/posts/popular.json", "/explore/posts/hot.json"] {
        let res = app.get(path).send().await.unwrap();
        assert_eq!(res.status(), 200, "{path}");
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["posts"].as_array().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn large_image_gets_sample() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let res = upload(
        &app,
        "admin",
        &key,
        png_bytes(1000, 900, 40),
        "big.png",
        &[],
    )
    .await;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let post = &body["post"];
    assert_eq!(post["sample"]["has"], true);
    assert!(post["sample"]["url"].as_str().unwrap().contains("/data/sample/"));
    assert!(post["sample"]["width"].as_i64().unwrap() <= 850);
    assert!(post["sample"]["height"].as_i64().unwrap() <= 850);
}

#[tokio::test]
async fn child_post_relationships() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let parent = seed_post(&app, "admin", &key, 50, "parent").await;
    let res = upload(
        &app,
        "admin",
        &key,
        png_bytes(16, 16, 51),
        "child.png",
        &[("post[parent_id]", &parent.to_string())],
    )
    .await;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let child_id = body["post_id"].as_i64().unwrap();

    let res = app
        .get(&format!("/posts/{parent}.json"))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["relationships"]["has_children"], true);
    assert_eq!(body["post"]["relationships"]["children"][0], child_id);

    assert_eq!(
        list_ids(&app, &[("tags", &format!("parent:{parent}")])).await,
        vec![child_id]
    );
    assert_eq!(
        list_ids(&app, &[("tags", "parent:any")]).await,
        vec![child_id]
    );
}
