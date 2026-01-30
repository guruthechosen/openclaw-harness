//! WebSocket handler for real-time events

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

use super::{AppState, WebEvent};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to events
    let mut rx = state.event_tx.subscribe();

    // Send initial status
    let status = WebEvent::Status {
        connected: true,
        monitoring: vec!["openclaw".to_string()],
    };
    if let Ok(json) = serde_json::to_string(&status) {
        let _ = sender.send(Message::Text(json)).await;
    }

    info!("🔌 WebSocket client connected");

    // Spawn task to forward events to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages (ping/pong, commands)
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Handle commands from client
                    if text == "ping" {
                        // Client ping - already handled by WebSocket layer
                    }
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!("🔌 WebSocket client disconnected");
}
