mod common;

use common::*;
use serde_json::Value;

#[tokio::test]
async fn first_user_becomes_admin_via_api() {
    let app = spawn().await;
    let res = app
        .request(reqwest::Method::POST, "/users.json")
        .form(&[("name", "owner"), ("password", "secret1")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["name"], "owner");
    assert_eq!(body["level"], 50);
    assert!(body["api_key"].as_str().unwrap().len() >= 32);
}

#[tokio::test]
async fn second_user_requires_admin() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);

    let res = app
        .request(reqwest::Method::POST, "/users.json")
        .form(&[("name", "newbie"), ("password", "secret1")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    let res = app
        .authed(reqwest::Method::POST, "/users.json", "admin", &key)
        .form(&[("name", "newbie"), ("password", "secret1"), ("level", "20")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["level"], 20);
}

#[tokio::test]
async fn create_user_validates_input() {
    let app = spawn().await;
    let res = app
        .request(reqwest::Method::POST, "/users.json")
        .form(&[("name", "x"), ("password", "12")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn login_sets_session_cookie() {
    let app = spawn().await;
    add_member(&app);

    let res = app
        .request(reqwest::Method::POST, "/session.json")
        .json(&serde_json::json!({"name": "member", "password": "memberpw"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let cookie = res
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(cookie.contains("self26_session="));
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["success"], true);
    assert_eq!(body["user"]["name"], "member");
    assert!(body["user"]["api_key"].is_string());

    let token = cookie.split(';').next().unwrap().to_string();
    let res = app
        .get("/users/current.json")
        .header("cookie", &token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["name"], "member");
}

#[tokio::test]
async fn login_rejects_bad_credentials() {
    let app = spawn().await;
    add_member(&app);
    for body in [
        serde_json::json!({"name": "member", "password": "wrong"}),
        serde_json::json!({"name": "ghost", "password": "memberpw"}),
    ] {
        let res = app
            .request(reqwest::Method::POST, "/session.json")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 401);
    }
}

#[tokio::test]
async fn logout_invalidates_session() {
    let app = spawn().await;
    add_member(&app);
    let res = app
        .request(reqwest::Method::POST, "/session.json")
        .json(&serde_json::json!({"name": "member", "password": "memberpw"}))
        .send()
        .await
        .unwrap();
    let cookie = res
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let res = app
        .request(reqwest::Method::DELETE, "/session.json")
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .get("/users/current.json")
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn api_key_auth_basic_and_query() {
    let app = spawn().await;
    let (_, key) = add_member(&app);

    let res = app
        .authed(reqwest::Method::GET, "/favorites.json", "member", &key)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .get("/favorites.json")
        .query(&[("login", "member"), ("api_key", &key)])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(reqwest::Method::GET, "/favorites.json", "member", "badkey")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn protected_routes_reject_anonymous() {
    let app = spawn().await;
    add_member(&app);
    for (method, path) in [
        (reqwest::Method::POST, "/favorites.json"),
        (reqwest::Method::GET, "/favorites.json"),
        (reqwest::Method::POST, "/comments.json"),
        (reqwest::Method::POST, "/notes.json"),
        (reqwest::Method::POST, "/pools.json"),
        (reqwest::Method::POST, "/posts/1/votes.json"),
    ] {
        let res = app.request(method, path).send().await.unwrap();
        assert_eq!(res.status(), 401, "{path}");
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["success"], false, "{path}");
    }
}

#[tokio::test]
async fn users_list_hides_api_keys_from_others() {
    let app = spawn().await;
    add_admin(&app);
    let (_, mkey) = add_member(&app);

    let res = app.get("/users.json").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    for user in body.as_array().unwrap() {
        assert!(user["api_key"].is_null(), "key leaked to anonymous");
    }

    let res = app
        .authed(reqwest::Method::GET, "/users.json", "member", &mkey)
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let me = body
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["name"] == "member")
        .unwrap();
    assert!(me["api_key"].is_string());
    let admin = body
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["name"] == "admin")
        .unwrap();
    assert!(admin["api_key"].is_null());
}

#[tokio::test]
async fn admin_reset_api_key() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (mid, mkey) = add_member(&app);

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/admin/api/users/{mid}/reset_key.json"),
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let new_key = body["api_key"].as_str().unwrap().to_string();
    assert_ne!(new_key, mkey);

    let res = app
        .authed(reqwest::Method::GET, "/favorites.json", "member", &mkey)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
    let res = app
        .authed(reqwest::Method::GET, "/favorites.json", "member", &new_key)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn member_cannot_reset_api_key() {
    let app = spawn().await;
    let (aid, _) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/admin/api/users/{aid}/reset_key.json"),
            "member",
            &mkey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}
