mod common;

use common::*;
use serde_json::Value;

#[tokio::test]
async fn pool_crud_and_membership() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let p1 = seed_post(&app, "member", &key, 1, "one").await;
    let p2 = seed_post(&app, "member", &key, 2, "two").await;

    let ids = format!("{p1} {p2}");
    let res = app
        .authed(reqwest::Method::POST, "/pools.json", "member", &key)
        .form(&[
            ("pool[name]", "My Cool Pool"),
            ("pool[description]", "desc"),
            ("pool[post_ids]", ids.as_str()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let pool_id = body["id"].as_i64().unwrap();
    assert_eq!(body["name"], "my_cool_pool");
    assert_eq!(body["category"], "series");
    assert_eq!(body["post_ids"].as_array().unwrap().len(), 2);
    assert_eq!(body["post_count"], 2);

    let res = app.get("/pools.json").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    let res = app
        .get(&format!("/pools/{pool_id}.json"))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post_ids"], serde_json::json!([p1, p2]));

    let res = app
        .get(&format!("/pools/{pool_id}/posts.json"))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["posts"].as_array().unwrap().len(), 2);

    let res = app
        .get(&format!("/posts/{p1}/pools.json"))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap()[0]["id"], pool_id);

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/pools/{pool_id}.json"),
            "member",
            &key,
        )
        .form(&[
            ("pool[post_ids]", &p2.to_string()),
            ("pool[description]", &"updated".to_string()),
            ("pool[category]", &"collection".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post_ids"], serde_json::json!([p2]));
    assert_eq!(body["description"], "updated");
    assert_eq!(body["category"], "collection");

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/pools/{pool_id}.json"),
            "member",
            &key,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let res = app.get("/pools.json").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn pool_search_metatag() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let in_pool = seed_post(&app, "member", &key, 3, "inside").await;
    seed_post(&app, "member", &key, 4, "outside").await;

    app.authed(reqwest::Method::POST, "/pools.json", "member", &key)
        .form(&[
            ("pool[name]", "story".to_string()),
            ("pool[post_ids]", in_pool.to_string()),
        ])
        .send()
        .await
        .unwrap();

    let res = app
        .get("/posts.json")
        .query(&[("tags", "pool:story")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["id"], in_pool);
}

#[tokio::test]
async fn fav_search_metatag() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let fav = seed_post(&app, "member", &key, 5, "liked").await;
    seed_post(&app, "member", &key, 6, "meh").await;

    app.authed(reqwest::Method::POST, "/favorites.json", "member", &key)
        .form(&[("post_id", fav.to_string())])
        .send()
        .await
        .unwrap();

    let res = app
        .get("/posts.json")
        .query(&[("tags", "fav:member")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let posts = body["posts"].as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["id"], fav);
}

#[tokio::test]
async fn tags_listing_and_search() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    seed_post(&app, "admin", &key, 7, "cat cute").await;

    let res = app.get("/tags.json").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let tags = body.as_array().unwrap();
    assert!(tags
        .iter()
        .any(|t| t["name"] == "cat" && t["post_count"] == 1));

    let res = app
        .get("/tags.json")
        .query(&[("search[name_matches]", "ca*")])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let names: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["cat"]);

    let res = app.get("/tags/cat.json").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["name"], "cat");
    assert_eq!(body["post_count"], 1);

    let res = app
        .get("/tags/autocomplete.json")
        .query(&[("search[query]", "cu")])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap()[0]["value"], "cute");

    let res = app.get("/tags/popular.json").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert!(body["tags"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn tag_category_update_shows_in_v2() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let id = seed_post(&app, "admin", &key, 8, "my_artist").await;

    let res = app.get("/tags.json").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    let tag_id = body
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "my_artist")
        .unwrap()["id"]
        .as_i64()
        .unwrap();

    let res = app
        .authed(
            reqwest::Method::PATCH,
            &format!("/tags/{tag_id}.json"),
            "admin",
            &key,
        )
        .form(&[("category", "1")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["category"], 1);

    let res = app
        .get(&format!("/posts/{id}.json"))
        .query(&[("v2", "true"), ("mode", "extended")])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["tags"]["artist"][0], "my_artist");
}

#[tokio::test]
async fn tag_aliases_and_implications() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);

    let res = app
        .authed(
            reqwest::Method::POST,
            "/admin/api/relations/aliases.json",
            "admin",
            &key,
        )
        .form(&[("antecedent", "kitty"), ("consequent", "cat")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(
            reqwest::Method::POST,
            "/admin/api/relations/implications.json",
            "admin",
            &key,
        )
        .form(&[("antecedent", "cat"), ("consequent", "feline")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app.get("/tag_aliases.json").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap()[0]["antecedent_name"], "kitty");
    assert_eq!(body.as_array().unwrap()[0]["consequent_name"], "cat");

    let res = app.get("/tag_implications.json").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap()[0]["antecedent_name"], "cat");
    assert_eq!(body.as_array().unwrap()[0]["consequent_name"], "feline");

    let res = upload(
        &app,
        "admin",
        &key,
        png_bytes(16, 16, 60),
        "x.png",
        &[("post[tag_string]", "kitty")],
    )
    .await;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let tag_string = body["post"]["tag_string"].as_str().unwrap();
    assert!(tag_string.contains("cat"), "{tag_string}");
    assert!(tag_string.contains("feline"), "{tag_string}");
    assert!(!tag_string.contains("kitty"));
}

#[tokio::test]
async fn tag_relations_require_admin() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let res = app
        .authed(
            reqwest::Method::POST,
            "/admin/api/relations/aliases.json",
            "member",
            &key,
        )
        .form(&[("antecedent", "a"), ("consequent", "b")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn notes_crud() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 9, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/notes.json", "member", &key)
        .form(&[
            ("note[post_id]", &id.to_string()),
            ("note[x]", &"10".to_string()),
            ("note[y]", &"20".to_string()),
            ("note[width]", &"30".to_string()),
            ("note[height]", &"40".to_string()),
            ("note[body]", &"translated text".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let nid = body["id"].as_i64().unwrap();
    assert_eq!(body["x"], 10);
    assert_eq!(body["body"], "translated text");
    assert_eq!(body["is_active"], true);

    let res = app
        .get("/notes.json")
        .query(&[("search[post_id]", id.to_string())])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    let res = app.get(&format!("/notes/{nid}.json")).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post_id"], id);

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/notes/{nid}.json"),
            "member",
            &key,
        )
        .form(&[("note[body]", "revised"), ("note[x]", "99")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["body"], "revised");
    assert_eq!(body["x"], 99);

    let res = app.get(&format!("/posts/{id}.json")).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["has_notes"], true);

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/notes/{nid}.json"),
            "member",
            &key,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .get("/notes.json")
        .query(&[("search[post_id]", id.to_string())])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn notes_require_auth_and_valid_post() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 10, "cat").await;

    let res = app
        .request(reqwest::Method::POST, "/notes.json")
        .form(&[
            ("note[post_id]", &id.to_string()),
            ("note[body]", &"x".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);

    let res = app
        .authed(reqwest::Method::POST, "/notes.json", "member", &key)
        .form(&[("note[post_id]", "99999"), ("note[body]", "x")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}
