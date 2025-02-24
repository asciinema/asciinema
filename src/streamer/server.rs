use super::alis;
use super::session;
use axum::{
    extract::connect_info::ConnectInfo,
    extract::ws,
    extract::State,
    http::{header, StatusCode, Uri},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::sink;
use futures_util::{stream, StreamExt};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::future;
use std::io;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tower_http::trace;
use tracing::info;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

#[derive(Clone)]
struct AppState {
    clients_tx: mpsc::Sender<session::Client>,
    html_head: Option<String>,
}

pub async fn serve(
    listener: std::net::TcpListener,
    clients_tx: mpsc::Sender<session::Client>,
    shutdown_token: tokio_util::sync::CancellationToken,
    html_head: Option<String>,
) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;

    let trace = trace::TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::default().include_headers(true));

    let state = AppState {
        clients_tx,
        html_head,
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback(static_handler)
        .with_state(state)
        .layer(trace);

    let signal = async move {
        let _ = shutdown_token.cancelled().await;
    };

    info!(
        "HTTP server listening on {}",
        listener.local_addr().unwrap()
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(signal)
    .tcp_nodelay(true)
    .await
}

async fn static_handler(uri: Uri, State(state): State<AppState>) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');

    if path.is_empty() {
        path = "index.html";
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            if path == "index.html" {
                let content = String::from_utf8(content.data.clone().into())
                    .expect("index.html asset contains invalid UTF-8 string");

                let html_head = state.html_head.as_deref().unwrap_or("");
                let content = content.replace("{{{html_head}}}", html_head);

                return ([(header::CONTENT_TYPE, mime.as_ref())], content).into_response();
            }

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }

        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}

async fn ws_handler(
    ws: ws::WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.protocols(["v1.alis"])
        .on_upgrade(move |socket| async move {
            info!("websocket client {addr} connected");

            if socket.protocol().is_some() {
                let _ = handle_socket(socket, state.clients_tx).await;
                info!("websocket client {addr} disconnected");
            } else {
                info!("subprotocol negotiation failed, closing connection");
                close_socket(socket).await;
            }
        })
}

async fn handle_socket(
    socket: ws::WebSocket,
    clients_tx: mpsc::Sender<session::Client>,
) -> anyhow::Result<()> {
    let (sink, stream) = socket.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));
    let close_msg = close_message(ws::close_code::NORMAL, "Stream ended");

    let result = alis::stream(&clients_tx)
        .await?
        .map(ws_result)
        .chain(stream::once(future::ready(Ok(close_msg))))
        .forward(sink)
        .await;

    drainer.abort();
    result?;

    Ok(())
}

async fn close_socket(mut socket: ws::WebSocket) {
    let msg = close_message(ws::close_code::PROTOCOL, "Subprotocol negotiation failed");
    let _ = socket.send(msg).await;
}

fn close_message(code: ws::CloseCode, reason: &'static str) -> ws::Message {
    ws::Message::Close(Some(ws::CloseFrame {
        code,
        reason: Cow::from(reason),
    }))
}

fn ws_result(m: Result<Vec<u8>, BroadcastStreamRecvError>) -> Result<ws::Message, axum::Error> {
    match m {
        Ok(bytes) => Ok(ws::Message::Binary(bytes)),
        Err(e) => Err(axum::Error::new(e)),
    }
}
