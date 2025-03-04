use crate::state::ServerState;
use axum::{
    body::Full,
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{header::CONTENT_TYPE, Response},
    response::{Html, IntoResponse},
};
use std::{
    io::{self, ErrorKind},
    sync::Arc,
};
use tokio::fs;
use tracing::{debug, error};

pub async fn root(State(state): State<Arc<ServerState>>) -> Html<String> {
    include_str!("base.html")
        .replace("{addr}", &state.args.address)
        .replace("{port}", &state.args.port.to_string())
        .into()
}

pub async fn target(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let filename = if state.args.no_recompile {
        &state.args.filename
    } else {
        "output.pdf"
    };

    let data = match fs::read(filename).await {
        Ok(data) => data,
        Err(err) => {
            error!("Failed to read `{filename}` {err:?}");
            vec![]
        }
    };

    Response::builder()
        .header(CONTENT_TYPE, "application/pdf")
        .body(Full::from(data))
        .expect("Failed to build response")
}

pub async fn listen(
    State(state): State<Arc<ServerState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| hander(socket, state))
}

async fn hander(mut socket: WebSocket, state: Arc<ServerState>) {
    loop {
        state.changed.notified().await;
        debug!("Pdf recompiled, sending websocket event");

        if let Err(err) = socket
            .send(Message::Text("refresh".into()))
            .await
            .map_err(|e| e.into_inner())
        {
            match err.downcast_ref::<io::Error>() {
                Some(io) if io.kind() == ErrorKind::BrokenPipe => continue,
                _ => {}
            }

            error!("Failed to send message to the client: {err:?}")
        }
        debug!("Waiting for the next recompilation");
    }
}
