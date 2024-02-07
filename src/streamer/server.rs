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
use futures_util::{stream, StreamExt};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::future;
use std::io;
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub async fn serve(
    listener: std::net::TcpListener,
    clients_tx: mpsc::Sender<session::Client>,
    shutdown_rx: oneshot::Receiver<()>,
) -> io::Result<()> {
    listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(listener)?;

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(clients_tx)
        .fallback(static_handler);

    let signal = async {
        let _ = shutdown_rx.await;
    };

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
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    State(clients_tx): State<mpsc::Sender<session::Client>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let _ = handle_socket(socket, clients_tx).await;
    })
}

async fn handle_socket(
    socket: ws::WebSocket,
    clients_tx: mpsc::Sender<session::Client>,
) -> anyhow::Result<()> {
    alis::stream(&clients_tx)
        .await?
        .map(ws_result)
        .chain(stream::once(future::ready(Ok(close_message()))))
        .forward(socket)
        .await?;

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
