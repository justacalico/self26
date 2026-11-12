mod common;

use common::*;
use serde_json::Value;

#[tokio::test]
async fn flag_flow_and_admin_resolve() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "member", &mkey, 1, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/post_flags.json", "member", &mkey)
        .form(&[
            ("post_flag[post_id]", id.to_string()),
            ("post_flag[reason]", "breaks rules".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let fid = body["id"].as_i64().unwrap();
    assert_eq!(body["post_id"], id);
    assert_eq!(body["reason"], "breaks rules");
    assert_eq!(body["is_resolved"], false);

    let res = app.get(&format!("/posts/{id}.json")).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["flags"]["flagged"], true);

    let res = app.get("/post_flags.json").send().await.unwrap();
    assert_eq!(res.status(), 401);

    let res = app
        .authed(reqwest::Method::GET, "/post_flags.json", "admin", &akey)
        .query(&[("search[is_resolved]", "false")])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/admin/api/flags/{fid}/resolve.json"),
            "member",
            &mkey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/admin/api/flags/{fid}/resolve.json"),
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = app
        .authed(
            reqwest::Method::GET,
            &format!("/post_flags/{fid}.json"),
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["is_resolved"], true);

    let res = app.get(&format!("/posts/{id}.json")).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["post"]["flags"]["flagged"], false);
}

#[tokio::test]
async fn flag_requires_post_and_auth() {
    let app = spawn().await;
    let (_, mkey) = add_member(&app);

    let res = app
        .request(reqwest::Method::POST, "/post_flags.json")
        .form(&[("post_flag[post_id]", "1")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);

    let res = app
        .authed(reqwest::Method::POST, "/post_flags.json", "member", &mkey)
        .form(&[("post_flag[post_id]", "99999")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn admin_stats_counts() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    seed_post(&app, "member", &mkey, 2, "cat").await;

    let res = app.get("/admin/api/stats.json").send().await.unwrap();
    assert_eq!(res.status(), 401);

    let res = app
        .authed(
            reqwest::Method::GET,
            "/admin/api/stats.json",
            "member",
            &mkey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    let res = app
        .authed(
            reqwest::Method::GET,
            "/admin/api/stats.json",
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["posts"], 1);
    assert_eq!(body["users"], 2);
    assert_eq!(body["deleted_posts"], 0);
    assert!(body["disk_bytes"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn tickets_create_list_resolve() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "member", &mkey, 3, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/tickets.json", "member", &mkey)
        .form(&[
            ("ticket[qtype]", "post".to_string()),
            ("ticket[disp_id]", id.to_string()),
            ("ticket[reason]", "wrong tags".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let tid = body["id"].as_i64().unwrap();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["qtype"], "post");

    let res = app
        .authed(reqwest::Method::GET, "/tickets.json", "member", &mkey)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    let res = app
        .authed(reqwest::Method::GET, "/tickets.json", "admin", &akey)
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    let res = app
        .authed(
            reqwest::Method::POST,
            &format!("/admin/api/tickets/{tid}/resolve.json"),
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn admin_delete_flag() {
    let app = spawn().await;
    let (_, akey) = add_admin(&app);
    let (_, mkey) = add_member(&app);
    let id = seed_post(&app, "member", &mkey, 4, "cat").await;

    let res = app
        .authed(reqwest::Method::POST, "/post_flags.json", "member", &mkey)
        .form(&[("post_flag[post_id]", id.to_string())])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let fid = body["id"].as_i64().unwrap();

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/post_flags/{fid}.json"),
            "member",
            &mkey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    let res = app
        .authed(
            reqwest::Method::DELETE,
            &format!("/post_flags/{fid}.json"),
            "admin",
            &akey,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}
