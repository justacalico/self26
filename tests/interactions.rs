mod common;

use common::*;
use serde_json::Value;

#[tokio::test]
async fn favorites_add_list_remove() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 1, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/favorites.json", "member", &key)
        .form(&[("post_id", &id.to_string())])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(reqwest::Method::GET, "/favorites.json", "member", &key)
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["posts"].as_array().unwrap().len(), 1);
    assert_eq!(body["posts"][0]["id"], id);
    assert_eq!(body["posts"][0]["is_favorited"], true);
    assert_eq!(body["posts"][0]["fav_count"], 1);

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/favorites/{id}.json"),
            "member",
            &key,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(reqwest::Method::GET, "/favorites.json", "member", &key)
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert!(body["posts"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn favorites_json_body_and_v2() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 2, "dog").await;

    let res = app
        .authed(reqwest::Method::POST, "/favorites.json", "member", &key)
        .json(&serde_json::json!({"post_id": id}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(reqwest::Method::GET, "/favorites.json", "member", &key)
        .query(&[("v2", "true"), ("mode", "extended")])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let posts = body.as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["stats"]["is_favorited"], true);
}

#[tokio::test]
async fn favorite_missing_post_404s() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let res = app
        .authed(reqwest::Method::POST, "/favorites.json", "member", &key)
        .form(&[("post_id", "99999")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn post_votes_up_down_clear() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 3, "bird").await;

    for (score, expect_total, expect_ours) in [("up", 1, 1), ("down", -1, -1), ("0", 0, 0)] {
        let res = app
            .authed(
                reqwest::Method::POST,
                &format!("/posts/{id}/votes.json"),
                "member",
                &key,
            )
            .form(&[("score", score)])
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["score"], expect_total);
        assert_eq!(body["our_score"], expect_ours);
    }
}

#[tokio::test]
async fn vote_shows_up_in_post_json() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 4, "fox").await;

    app.authed(
        reqwest::Method::POST,
        &format!("/posts/{id}/votes.json"),
        "member",
        &key,
    )
    .json(&serde_json::json!({"score": 1}))
    .send()
    .await
    .unwrap();

    let res = app
        .authed(
            reqwest::Method::GET,
            &format!("/posts/{id}.json"),
            "member",
            &key,
        )
        .query(&[("v2", "true"), ("mode", "extended")])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["stats"]["vote"], 1);
    assert_eq!(body["stats"]["score"]["up"], 1);
    assert_eq!(body["stats"]["score"]["total"], 1);
}

#[tokio::test]
async fn vote_errors() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 5, "wolf").await;

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/posts/{id}/votes.json"),
            "member",
            &key,
        )
        .form(&[("score", "bogus")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);

    let res = app
        .authed(
            reqwest::Method::POST,
            "/posts/99999/votes.json",
            "member",
            &key,
        )
        .form(&[("score", "up")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);

    let res = app
        .request(reqwest::Method::POST, &format!("/posts/{id}/votes.json"))
        .form(&[("score", "up")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn comments_crud() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 6, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/comments.json", "member", &key)
        .form(&[
            ("comment[post_id]", &id.to_string()),
            ("comment[body]", &"nice one".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let cid = body["id"].as_i64().unwrap();
    assert_eq!(body["body"], "nice one");
    assert_eq!(body["post_id"], id);
    assert_eq!(body["creator_name"], "member");

    let res = app
        .get("/comments.json")
        .query(&[("search[post_id]", id.to_string())])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    let res = app
        .get(&format!("/comments/{cid}.json"))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["id"], cid);

    let res = app
        .authed(
            reqwest::Method::PATCH,
            &format!("/comments/{cid}.json"),
            "member",
            &key,
        )
        .form(&[("comment[body]", "edited")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["body"], "edited");

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/comments/{cid}.json"),
            "member",
            &key,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .get("/comments.json")
        .query(&[("search[post_id]", id.to_string())])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn comment_create_errors() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 7, "cat").await;

    for form in [
        vec![("comment[body]", "no post".to_string())],
        vec![
            ("comment[post_id]", "99999".to_string()),
            ("comment[body]", "ghost post".to_string()),
        ],
        vec![
            ("comment[post_id]", id.to_string()),
            ("comment[body]", "".to_string()),
        ],
    ] {
        let res = app
            .authed(reqwest::Method::POST, "/comments.json", "member", &key)
            .form(&form)
            .send()
            .await
            .unwrap();
        let status = res.status().as_u16();
        assert!(status == 400 || status == 404, "form {form:?} -> {status}");
    }
}

#[tokio::test]
async fn comment_votes_and_foreign_delete() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "member", &mkey, 8, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/comments.json", "member", &mkey)
        .form(&[
            ("comment[post_id]", &id.to_string()),
            ("comment[body]", &"hi".to_string()),
        ])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let cid = body["id"].as_i64().unwrap();

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/comments/{cid}/votes.json"),
            "admin",
            &akey,
        )
        .form(&[("score", "up")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["score"], 1);

    let (_, okey) = add_user(&app, "other", "otherpw", 20);
    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/comments/{cid}.json"),
            "other",
            &okey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/comments/{cid}.json"),
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn comments_json_body_create() {
    let app = spawn().await;
    let (_, key) = add_member(&app);
    let id = seed_post(&app, "member", &key, 9, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/comments.json", "member", &key)
        .json(&serde_json::json!({"comment": {"post_id": id, "body": "json hi"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["body"], "json hi");
    assert_eq!(body["post_id"], id);
}
