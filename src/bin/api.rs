use std::{
    collections::HashMap,
    io::BufWriter,
    path::{Path, PathBuf},
};

use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use image::codecs::gif::GifEncoder;
use sha2::Digest as _;
use sha2::Sha256;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{debug, info, Level};

struct APIError(anyhow::Error);

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for APIError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .init();

    let app = Router::new()
        .route("/", get(index))
        .route("/favicon.ico", get(favicon))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    let addr = listener.local_addr()?;
    info!(?addr, "listening");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, APIError> {
    let default = "pigeon".to_string();
    let text = params
        .get("q")
        .filter(|v| !v.is_empty())
        .unwrap_or(&default);
    let text = normalize_text(text);

    let headers = [("Content-Type", "image/gif")];
    let data = if let Some(data) = cache_read(text) {
        debug!("cache hit");
        data
    } else {
        debug!("cache miss, rendering");
        let data = render_gif(text)?;
        cache_write(text, &data)?;
        data
    };
    Ok((headers, data))
}

async fn favicon() -> impl IntoResponse {
    let headers = [("Content-Type", "image/gif")];
    (headers, sparkler::SPARKLES_DATA)
}

fn normalize_text(text: &str) -> &str {
    text.trim()
}

fn cache_path(text: &str) -> PathBuf {
    let key = {
        let mut hasher = Sha256::new();
        hasher.update(text);
        format!("{:x}", hasher.finalize())
    };
    let path = Path::new(".cache").join(key);
    debug!(?path, "cache key");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    path
}

fn cache_read(text: &str) -> Option<Vec<u8>> {
    let path = cache_path(text);
    if path.exists() {
        let data = std::fs::read(path).ok()?;
        Some(data)
    } else {
        None
    }
}

fn cache_write(text: &str, data: &[u8]) -> std::io::Result<()> {
    let path = cache_path(text);
    std::fs::write(path, data)
}

fn render_gif(text: &str) -> Result<Vec<u8>, APIError> {
    let frames = sparkler::render(text)?;
    let mut buf = BufWriter::new(Vec::new());
    let mut encoder = GifEncoder::new_with_speed(&mut buf, 10);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;
    encoder.encode_frames(frames)?;
    drop(encoder);
    Ok(buf.into_inner()?)
}
