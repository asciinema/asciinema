use std::future;
use std::io;
use std::net::SocketAddr;
use std::path::Path;

use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{self, CloseCode, CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{header, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::serve::ListenerExt;
use axum::Router;
use futures_util::{sink, StreamExt};
use rust_embed::RustEmbed;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_util::sync::CancellationToken;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::info;

use crate::alis;
use crate::stream::Subscriber;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub async fn serve(
    listener: std::net::TcpListener,
    subscriber: Subscriber,
    shutdown_token: CancellationToken,
) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;

    let trace =
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(subscriber)
        .fallback(static_handler)
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

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(signal)
    .await
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');

    if path.is_empty() {
        path = "index.html";
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_from_path(path);

            ([(header::CONTENT_TYPE, mime)], content.data).into_response()
        }

        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}

fn mime_from_path(path: &str) -> &str {
    let lowercase_path = &path.to_lowercase();

    let ext = Path::new(lowercase_path)
        .extension()
        .and_then(|e| e.to_str());

    match ext {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some(_) | None => "application/octet-stream",
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(subscriber): State<Subscriber>,
) -> impl IntoResponse {
    ws.protocols(["v1.alis"])
        .on_upgrade(move |socket| async move {
            info!("websocket client {addr} connected");

            if socket.protocol().is_some() {
                let _ = handle_socket(socket, subscriber).await;
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
        .await?
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
