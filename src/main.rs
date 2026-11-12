use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use self26::media::MediaProcessor;
use self26::{app, auth, db, models::StoredUser, AppState};
use tracing_subscriber::EnvFilter;

struct Cli {
    port: u16,
    port_set: bool,
    host: String,
    data_dir: Option<PathBuf>,
    base_url: String,
    dev: bool,
    local: bool,
    admin_password: Option<String>,
}

fn parse_args() -> Cli {
    let mut cli = Cli {
        port: 8021,
        port_set: false,
        host: "0.0.0.0".into(),
        data_dir: None,
        base_url: String::new(),
        dev: false,
        local: false,
        admin_password: std::env::var("SELF26_ADMIN_PASSWORD").ok(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dev" => cli.dev = true,
            "--local" => cli.local = true,
            "--port" | "-p" => {
                if let Some(v) = args.next() {
                    cli.port = v.parse().unwrap_or(cli.port);
                    cli.port_set = true;
                }
            }
            "--host" => {
                if let Some(v) = args.next() {
                    cli.host = v;
                }
            }
            "--data-dir" | "-d" => {
                if let Some(v) = args.next() {
                    cli.data_dir = Some(PathBuf::from(v));
                }
            }
            "--base-url" => {
                if let Some(v) = args.next() {
                    cli.base_url = v;
                }
            }
            "--admin-password" => {
                if let Some(v) = args.next() {
                    cli.admin_password = Some(v);
                }
            }
            "--help" | "-h" => {
                println!(
                    "self26 — self-hosted e621-compatible media server\n\n\
                     Usage: self26 [OPTIONS]\n\n\
                     Options:\n\
                     \x20 --dev              dev mode (random port if unset)\n\
                     \x20 --local            loopback only, no auth, in-memory db (dev)\n\
                     \x20 --port, -p <PORT>  listen port (default 8021)\n\
                     \x20 --host <HOST>      bind address (default 0.0.0.0)\n\
                     \x20 --data-dir, -d <D> data directory\n\
                     \x20 --base-url <URL>   public base URL for file links\n\
                     \x20 --admin-password <PW>  initial admin password\n\
                     \x20 --help, -h         show this help"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    if cli.local {
        cli.host = "127.0.0.1".into();
        cli.dev = true;
    }
    if cli.dev && !cli.port_set {
        cli.port = 0;
    }
    cli
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = parse_args();
    let data_dir = cli.data_dir.unwrap_or_else(self26::default_data_dir);
    std::fs::create_dir_all(&data_dir)?;

    let db = if cli.local {
        db::open(None)?
    } else {
        db::open(Some(&data_dir.join("self26.db")))?
    };

    // First run: create the admin account.
    {
        let conn = db.lock().expect("db lock");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap_or(0);
        if count == 0 {
            let password = cli.admin_password.clone().unwrap_or_else(|| {
                if cli.local {
                    "admin".into()
                } else {
                    let mut buf = [0u8; 12];
                    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut buf);
                    hex::encode(buf)
                }
            });
            auth::create_user(&conn, "admin", &password, StoredUser::LEVEL_ADMIN)?;
            tracing::info!("created admin account 'admin'");
            if cli.admin_password.is_none() {
                tracing::info!("initial admin password: {password}");
            }
        }
    }

    let media = if cli.local {
        MediaProcessor::in_memory()
    } else {
        MediaProcessor::new(data_dir.join("media"))
    };

    let state = AppState {
        db,
        media: Arc::new(media),
        base_url: cli.base_url.clone(),
        local_mode: cli.dev,
    };

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("self26 listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app(state).into_make_service()).await?;
    Ok(())
}
