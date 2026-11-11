mod common;

use serde_json::Value;

#[tokio::test]
async fn health_reports_ok_and_counts() {
    let app = common::spawn().await;
    let res = app.get("/health.json").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["posts"], 0);
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn live_and_ready_return_ok() {
    let app = common::spawn().await;
    for path in ["/live", "/ready"] {
        let res = app.get(path).send().await.unwrap();
        assert_eq!(res.status(), 200, "{path}");
        assert_eq!(res.text().await.unwrap(), "ok");
    }
}

#[tokio::test]
async fn metrics_exposes_counters() {
    let app = common::spawn().await;
    let res = app.get("/metrics").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let text = res.text().await.unwrap();
    for key in ["self26_posts", "self26_users", "self26_tags", "self26_comments"] {
        assert!(text.contains(key), "missing {key} in {text}");
    }
}

#[tokio::test]
async fn well_known_identity() {
    let app = common::spawn().await;
    let res = app.get("/.well-known/self26").send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["name"], "self26");
    assert_eq!(body["api"], "e621-compatible");
    assert_eq!(body["v2"], true);
}

#[tokio::test]
async fn root_serves_spa_and_unknown_api_path_404s() {
    let app = common::spawn().await;
    let res = app.get("/").send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/html"
    );

    let res = app.get("/nope.json").send().await.unwrap();
    assert_eq!(res.status(), 404);
}
