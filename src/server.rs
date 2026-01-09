use std::cmp;
use std::collections::HashMap;
use std::future;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::LazyLock;

use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{self, CloseCode, CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::serve::ListenerExt;
use axum::Router;
use bytes::Bytes;
use futures_util::{sink, StreamExt};
use rust_embed::RustEmbed;
use tokio::time::{self, Duration};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tower_http::compression::CompressionLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::info;

use crate::alis;
use crate::hash;
use crate::stream::Subscriber;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

struct AssetInfo {
    bytes: Bytes,
    mime: &'static str,
    etag: String,
    digest_path: Option<String>,
}

struct AssetManifest {
    by_path: HashMap<String, AssetInfo>,
    by_digest: HashMap<String, String>,
    index_html: Bytes,
    index_etag: String,
}

const DIGEST_HEX_LEN: usize = 16;

static MANIFEST: LazyLock<AssetManifest> = LazyLock::new(|| {
    let mut by_path = HashMap::new();
    let mut by_digest = HashMap::new();

    for path in Assets::iter() {
        let file = Assets::get(&path).unwrap(); // safe because we iterate over embedded assets
        let bytes = Bytes::from(file.data.into_owned());
        let mime = mime_from_path(&path);
        let mut hash_hex = hex_encode(&file.metadata.sha256_hash());
        hash_hex.truncate(DIGEST_HEX_LEN);
        let etag = format!("W/\"{hash_hex}\"");
        let digest_path = digest_path(&path, &hash_hex);

        if let Some(digest_path) = &digest_path {
            by_digest.insert(digest_path.clone(), path.to_string());
        }

        let asset_info = AssetInfo {
            bytes,
            mime,
            etag,
            digest_path,
        };

        by_path.insert(path.to_string(), asset_info);
    }

    let index_html = rewrite_index_html(&by_path);
    let index_etag = format!("W/\"{:032x}\"", hash::fnv1a_128(&index_html));

    AssetManifest {
        by_path,
        by_digest,
        index_html,
        index_etag,
    }
});

#[derive(Clone)]
struct AppState {
    subscriber: Subscriber,
    tracker: TaskTracker,
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    subscriber: Subscriber,
    shutdown_token: CancellationToken,
) -> io::Result<()> {
    let trace =
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true));

    let tracker = TaskTracker::new();

    let state = AppState {
        subscriber,
        tracker: tracker.clone(),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
        .fallback(static_handler)
        .layer(CompressionLayer::new())
        .layer(trace);

    let signal = async move {
        let _ = shutdown_token.cancelled().await;
    };

    info!(
        "HTTP server listening on {}",
        listener.local_addr().unwrap()
    );

    let listener = listener.tap_io(|tcp_stream| {
        let _ = tcp_stream.set_nodelay(true);
    });

    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(signal)
    .await;

    tracker.close();
    let _ = time::timeout(Duration::from_secs(3), tracker.wait()).await;

    result
}

async fn static_handler(headers: HeaderMap, uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');

    if path.is_empty() {
        path = "index.html";
    }

    let (resolved_path, is_digest) = if let Some(original) = MANIFEST.by_digest.get(path) {
        (original.as_str(), true)
    } else {
        (path, false)
    };

    if resolved_path == "index.html" {
        return asset_response(
            &headers,
            MANIFEST.index_html.clone(),
            "text/html",
            &MANIFEST.index_etag,
            "public, max-age=0, must-revalidate",
        );
    }

    match MANIFEST.by_path.get(resolved_path) {
        Some(asset) => {
            let cache_control = if is_digest {
                "public, max-age=31536000, immutable"
            } else {
                "public, max-age=0, must-revalidate"
            };

            asset_response(
                &headers,
                asset.bytes.clone(),
                asset.mime,
                &asset.etag,
                cache_control,
            )
        }

        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}

fn mime_from_path(path: &str) -> &'static str {
    let lowercase_path = &path.to_lowercase();

    let ext = Path::new(lowercase_path)
        .extension()
        .and_then(|e| e.to_str());

    match ext {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("woff2") => "font/woff2",
        Some(_) | None => "application/octet-stream",
    }
}

fn asset_response(
    headers: &HeaderMap,
    body: Bytes,
    mime: &'static str,
    etag: &str,
    cache_control: &'static str,
) -> axum::response::Response {
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));

    resp_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );

    resp_headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    resp_headers.insert(header::ETAG, HeaderValue::from_str(etag).unwrap());

    if etag_matches(headers, etag) {
        return (StatusCode::NOT_MODIFIED, resp_headers).into_response();
    }

    (resp_headers, body).into_response()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn digest_path(path: &str, hash: &str) -> Option<String> {
    if path == "index.html" {
        return None;
    }

    match path.rsplit_once('.') {
        Some((base, ext)) => Some(format!("{base}.{hash}.{ext}")),
        None => Some(format!("{path}.{hash}")),
    }
}

fn rewrite_index_html(by_path: &HashMap<String, AssetInfo>) -> Bytes {
    let asset = by_path
        .get("index.html")
        .expect("assets/index.html missing");

    let mut html = String::from_utf8_lossy(&asset.bytes).into_owned();

    let mut mappings: Vec<(&str, &str)> = by_path
        .iter()
        .filter_map(|(path, info)| info.digest_path.as_deref().map(|d| (path.as_str(), d)))
        .collect();

    // Replace longest paths first to avoid overlapping replacements.
    mappings.sort_by_key(|(path, _)| cmp::Reverse(path.len()));

    for (path, digest) in mappings {
        html = html.replace(path, digest);
    }

    Bytes::from(html)
}

fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
    let Some(value) = headers.get(header::IF_NONE_MATCH) else {
        return false;
    };

    let Ok(value) = value.to_str() else {
        return false;
    };

    let etag_norm = etag.trim().trim_start_matches("W/");

    value.split(',').any(|candidate| {
        let candidate = candidate.trim();

        if candidate == "*" {
            return true;
        }

        candidate.trim_start_matches("W/") == etag_norm
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.protocols(["v1.alis"])
        .on_upgrade(move |socket| async move {
            info!("websocket client {addr} connected");

            if socket.protocol().is_some() {
                let _ = state
                    .tracker
                    .track_future(handle_socket(socket, state.subscriber))
                    .await;

                info!("websocket client {addr} disconnected");
            } else {
                info!("subprotocol negotiation failed, closing connection");
                close_socket(socket).await;
            }
        })
}

async fn handle_socket(socket: WebSocket, subscriber: Subscriber) -> anyhow::Result<()> {
    let (sink, stream) = socket.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));
    let close_msg = close_message(ws::close_code::NORMAL, "Stream ended");
    let stream = subscriber.subscribe().await?;

    let result = alis::stream(stream)
        .map(ws_result)
        .chain(futures_util::stream::once(future::ready(Ok(close_msg))))
        .forward(sink)
        .await;

    drainer.abort();
    result?;

    Ok(())
}

async fn close_socket(mut socket: WebSocket) {
    let msg = close_message(ws::close_code::PROTOCOL, "Subprotocol negotiation failed");
    let _ = socket.send(msg).await;
}

fn close_message(code: CloseCode, reason: &'static str) -> Message {
    Message::Close(Some(CloseFrame {
        code,
        reason: reason.into(),
    }))
}

fn ws_result(m: Result<Vec<u8>, BroadcastStreamRecvError>) -> Result<Message, axum::Error> {
    match m {
        Ok(bytes) => Ok(Message::Binary(bytes.into())),
        Err(e) => Err(axum::Error::new(e)),
    }
}
