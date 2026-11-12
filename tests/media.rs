mod common;

use common::*;
use serde_json::Value;

#[tokio::test]
async fn serves_original_with_headers() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let png = png_bytes(32, 24, 1);
    let res = upload(&app, "admin", &key, png.clone(), "a.png", &[]).await;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    let url = body["post"]["file"]["url"].as_str().unwrap().to_string();

    let res = app.client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers().get("content-type").unwrap(), "image/png");
    let cache = res
        .headers()
        .get("cache-control")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(cache.contains("immutable"));
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.as_ref(), png.as_slice());
}

#[tokio::test]
async fn serves_preview_and_sample_jpegs() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let res = upload(&app, "admin", &key, png_bytes(1000, 800, 2), "b.png", &[]).await;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();

    let preview = body["post"]["preview"]["url"].as_str().unwrap().to_string();
    let res = app.client.get(&preview).send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers().get("content-type").unwrap(), "image/jpeg");

    let sample = body["post"]["sample"]["url"].as_str().unwrap().to_string();
    assert!(sample.contains("/data/sample/"));
    let res = app.client.get(&sample).send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers().get("content-type").unwrap(), "image/jpeg");
}

#[tokio::test]
async fn missing_media_404s() {
    let app = spawn().await;
    for path in [
        "/data/original/ab/0123456789abcdef0123456789abcdef.png",
        "/data/preview/ab/0123456789abcdef0123456789abcdef.jpg",
        "/data/thumb/ab/x.png",
        "/data/original/abc/x.png",
        "/data/original/../secret.txt",
    ] {
        let res = app.get(path).send().await.unwrap();
        assert_eq!(res.status(), 404, "{path}");
    }
}

#[tokio::test]
async fn dot_segment_prefix_cannot_escape_media_dir() {
    let app = spawn().await;
    let media_root = app.state.media.data_dir.clone();
    std::fs::create_dir_all(&media_root).unwrap();
    std::fs::write(media_root.join("secret.txt"), b"hunter2").unwrap();

    for path in [
        "/data/original/../secret.txt",
        "/data/original/%2e%2e/secret.txt",
        "/data/preview/../secret.txt",
    ] {
        let status = raw_status(app.addr, path).await;
        assert_eq!(status, 404, "{path}");
    }
}

#[tokio::test]
async fn media_url_uses_request_host() {
    let app = spawn().await;
    let (_, key) = add_admin(&app);
    let res = upload(&app, "admin", &key, png_bytes(8, 8, 3), "c.png", &[]).await;
    let body: Value = res.json().await.unwrap();
    let url = body["post"]["file"]["url"].as_str().unwrap();
    assert!(url.starts_with(&app.base), "{url}");
}
