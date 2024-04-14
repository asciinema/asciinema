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
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tower_http::trace;
use tracing::info;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub async fn serve(
    listener: std::net::TcpListener,
    clients_tx: mpsc::Sender<session::Client>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;

    let trace = trace::TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::default().include_headers(true));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(clients_tx)
        .fallback(static_handler)
        .layer(trace);

    let signal = async move {
        let _ = shutdown_rx.recv().await;
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
    .await
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');

    if path.is_empty() {
        path = "index.html";
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }

        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}

async fn ws_handler(
    ws: ws::WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(clients_tx): State<mpsc::Sender<session::Client>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        info!("websocket client {addr} connected");
        let _ = handle_socket(socket, clients_tx).await;
        info!("websocket client {addr} disconnected");
    })
}

async fn handle_socket(
    socket: ws::WebSocket,
    clients_tx: mpsc::Sender<session::Client>,
) -> anyhow::Result<()> {
    let (sink, stream) = socket.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));

    let result = alis::stream(&clients_tx)
        .await?
        .map(ws_result)
        .chain(stream::once(future::ready(Ok(close_message()))))
        .forward(sink)
        .await;

    drainer.abort();
    result?;

    Ok(())
}

fn close_message() -> ws::Message {
    ws::Message::Close(Some(ws::CloseFrame {
        code: ws::close_code::NORMAL,
        reason: Cow::from("ended"),
    }))
}

fn ws_result(m: Result<Vec<u8>, BroadcastStreamRecvError>) -> Result<ws::Message, axum::Error> {
    match m {
        Ok(bytes) => Ok(ws::Message::Binary(bytes)),
        Err(e) => Err(axum::Error::new(e)),
    }
}
