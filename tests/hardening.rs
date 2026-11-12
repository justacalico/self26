mod common;

use common::*;
use serde_json::Value;

#[tokio::test]
async fn username_uniqueness_is_case_insensitive() {
    let app = spawn().await;
    add_user(&app, "Dupe", "pw", 20);

    let conn = app.state.db.lock().unwrap();
    let err = self26::auth::create_user(&conn, "dupe", "pw", 20);
    assert!(err.is_err());
}

#[tokio::test]
async fn basic_auth_with_password_rejected() {
    let app = spawn().await;
    add_user(&app, "member", "memberpw", 20);

    let res = app
        .authed(
            reqwest::Method::GET,
            "/post_flags.json",
            "member",
            "memberpw",
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn locked_tags_kept_on_member_edit() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "admin", &akey, 1, "cat keepme").await;

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/posts/{id}.json"),
            "admin",
            &akey,
        )
        .form(&[("locked_tags", "keepme")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/posts/{id}.json"),
            "member",
            &mkey,
        )
        .form(&[("post[tag_string]", "cat dog")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app.get(&format!("/posts/{id}.json")).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    let tags = body["post"]["tags"]["general"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(tags.contains(&"keepme".to_string()));
    assert!(tags.contains(&"dog".to_string()));
}

#[tokio::test]
async fn member_can_edit_tags_but_not_rating() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "admin", &akey, 1, "cat").await;

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/posts/{id}.json"),
            "member",
            &mkey,
        )
        .form(&[("post[tag_string]", "cat dog")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/posts/{id}.json"),
            "member",
            &mkey,
        )
        .form(&[("post[rating]", "e")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn mutations_rejected_on_deleted_post() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "admin", &akey, 1, "cat").await;

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/posts/{id}.json"),
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/posts/{id}/votes.json"),
            "member",
            &mkey,
        )
        .form(&[("score", "up")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);

    let res = app
        .authed(reqwest::Method::POST, "/favorites.json", "member", &mkey)
        .form(&[("post_id", &id.to_string())])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);

    let res = app
        .authed(reqwest::Method::POST, "/comments.json", "member", &mkey)
        .form(&[
            ("comment[post_id]", &id.to_string()),
            ("comment[body]", &"hi".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn comment_self_vote_rejected_and_unvote_works() {
    let app = spawn().await;
    let (_, mkey) = add_member(&app);
    let (_, okey) = add_user(&app, "other", "otherpw", 20);
    let id = seed_post(&app, "member", &mkey, 1, "cat").await;

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
            "member",
            &mkey,
        )
        .form(&[("score", "up")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/comments/{cid}/votes.json"),
            "other",
            &okey,
        )
        .form(&[("score", "up")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/comments/{cid}/votes.json"),
            "other",
            &okey,
        )
        .form(&[("score", "0")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["score"], 0);
}

#[tokio::test]
async fn flags_scoped_to_creator_for_members() {
    let app = spawn().await;
    let (_, mkey) = add_member(&app);
    let (_, okey) = add_user(&app, "other", "otherpw", 20);
    let id = seed_post(&app, "member", &mkey, 1, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/post_flags.json", "member", &mkey)
        .form(&[
            ("post_flag[post_id]", &id.to_string()),
            ("post_flag[reason]", &"mine".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let fid = body["id"].as_i64().unwrap();

    let res = app
        .authed(reqwest::Method::POST, "/post_flags.json", "other", &okey)
        .form(&[
            ("post_flag[post_id]", &id.to_string()),
            ("post_flag[reason]", &"theirs".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);

    let res = app
        .authed(reqwest::Method::GET, "/post_flags.json", "member", &mkey)
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let flags = body.as_array().unwrap();
    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0]["reason"], "mine");

    let res = app
        .authed(
            reqwest::Method::GET,
            &format!("/post_flags/{fid}.json"),
            "other",
            &okey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn status_locked_blocks_owner_delete() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "member", &mkey, 1, "cat").await;

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/posts/{id}.json"),
            "admin",
            &akey,
        )
        .form(&[("is_status_locked", "1")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/posts/{id}.json"),
            "member",
            &mkey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn random_post_honors_tags() {
    let app = spawn().await;
    let (_, mkey) = add_member(&app);
    seed_post(&app, "member", &mkey, 1, "alpha").await;
    seed_post(&app, "member", &mkey, 2, "beta").await;

    for _ in 0..5 {
        let res = app
            .get("/posts/random.json?tags=beta")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let body: Value = res.json().await.unwrap();
        let tags = body["post"]["tags"]["general"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert!(tags.contains(&"beta".to_string()));
    }
}

#[tokio::test]
async fn pool_rename_to_empty_rejected() {
    let app = spawn().await;
    let (_, mkey) = add_member(&app);

    let res = app
        .authed(reqwest::Method::POST, "/pools.json", "member", &mkey)
        .form(&[("pool[name]", "my pool")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let pid = body["id"].as_i64().unwrap();

    let res = app
        .authed(
            reqwest::Method::PUT,
            &format!("/pools/{pid}.json"),
            "member",
            &mkey,
        )
        .form(&[("pool[name]", "   ")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn ticket_creation_requires_auth() {
    let app = spawn().await;
    let res = app
        .request(reqwest::Method::POST, "/tickets.json")
        .form(&[("ticket[qtype]", "post"), ("ticket[reason]", "x")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn deleted_post_tag_counts_dropped() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let id = seed_post(&app, "admin", &akey, 1, "uniquetag").await;

    let res = app.get("/tags.json?name=uniquetag").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body[0]["post_count"], 1);

    app.authed(
        reqwest::Method::DELETE,
        &format!("/posts/{id}.json"),
        "admin",
        &akey,
    )
    .send()
    .await
    .unwrap();

    let res = app.get("/tags.json?name=uniquetag").send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body[0]["post_count"], 0);
}
