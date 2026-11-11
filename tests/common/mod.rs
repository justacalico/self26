use std::net::SocketAddr;
use std::sync::Arc;

use serde_json::Value;
use self26::media::MediaProcessor;
use self26::models::StoredUser;
use self26::{app, auth, db, AppState};

pub struct TestApp {
    pub addr: SocketAddr,
    pub base: String,
    pub client: reqwest::Client,
    pub state: AppState,
    pub _dir: tempfile::TempDir,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.get(self.url(path))
    }

    pub fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client.request(method, self.url(path))
    }

    pub fn authed(
        &self,
        method: reqwest::Method,
        path: &str,
        login: &str,
        key: &str,
    ) -> reqwest::RequestBuilder {
        self.request(method, path).basic_auth(login, Some(key))
    }
}

pub async fn spawn() -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let db = db::open(Some(&dir.path().join("test.db"))).unwrap();
    let media = MediaProcessor::new(dir.path().join("media"));
    let state = AppState {
        db,
        media: Arc::new(media),
        base_url: String::new(),
        local_mode: false,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, router.into_make_service())
            .await
            .unwrap();
    });
    TestApp {
        addr,
        base: format!("http://{addr}"),
        client: reqwest::Client::new(),
        state,
        _dir: dir,
    }
}

pub fn add_user(app: &TestApp, name: &str, password: &str, level: i64) -> (i64, String) {
    let conn = app.state.db.lock().unwrap();
    let id = auth::create_user(&conn, name, password, level).unwrap();
    let key = auth::get_user(&conn, id).unwrap().api_key;
    (id, key)
}

pub fn add_admin(app: &TestApp) -> (i64, String) {
    add_user(app, "admin", "adminpw", StoredUser::LEVEL_ADMIN)
}

pub fn add_member(app: &TestApp) -> (i64, String) {
    add_user(app, "member", "memberpw", StoredUser::LEVEL_MEMBER)
}

pub fn png_bytes(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([
            (x % 256) as u8 ^ seed,
            (y % 256) as u8,
            seed.wrapping_mul(37),
        ])
    });
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    buf.into_inner()
}

pub fn jpeg_bytes(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([
            seed,
            (x % 256) as u8,
            (y % 256) as u8 ^ seed,
        ])
    });
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .unwrap();
    buf.into_inner()
}

pub async fn upload(
    app: &TestApp,
    login: &str,
    key: &str,
    bytes: Vec<u8>,
    filename: &str,
    fields: &[(&str, &str)],
) -> reqwest::Response {
    let mut form = reqwest::multipart::Form::new().part(
        "post[file]",
        reqwest::multipart::Part::bytes(bytes).file_name(filename.to_string()),
    );
    for (k, v) in fields {
        form = form.text((*k).to_string(), (*v).to_string());
    }
    app.client
        .post(app.url("/posts.json"))
        .basic_auth(login, Some(key))
        .multipart(form)
        .send()
        .await
        .unwrap()
}

pub async fn seed_post(app: &TestApp, login: &str, key: &str, seed: u8, tags: &str) -> i64 {
    let res = upload(
        app,
        login,
        key,
        png_bytes(64, 48, seed),
        "test.png",
        &[("post[tag_string]", tags), ("post[rating]", "s")],
    )
    .await;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await.unwrap();
    body["post_id"].as_i64().unwrap()
}

/// Raw HTTP/1.1 GET over a socket for paths reqwest's Url parser would
/// normalize away (dot segments, percent-encoded dots).
pub async fn raw_status(addr: SocketAddr, path: &str) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let head = String::from_utf8_lossy(&buf);
    head.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
