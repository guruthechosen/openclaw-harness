//! API Proxy — sits between Clawdbot and Anthropic API
//!
//! Intercepts responses and blocks dangerous tool_use actions.

pub mod config;
pub mod interceptor;
pub mod streaming;

use self::config::{ProxyConfig, ProxyMode};
use self::interceptor::{intercept_response, format_telegram_alert, InterceptResult};
use self::streaming::{StreamInterceptor, SseLineBuffer, parse_sse_events};
use crate::rules::{default_rules, Rule, RuleAction};
use crate::{AlertConfig, TelegramConfig};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use futures_util::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

/// Shared state for the proxy
struct ProxyState {
    client: Client,
    target: String,
    rules: Vec<Rule>,
    mode: ProxyMode,
    telegram: Option<TelegramConfig>,
}

/// Start the proxy server
pub async fn start_proxy(config: ProxyConfig, alert_config: Option<AlertConfig>) -> anyhow::Result<()> {
    let mut rules = default_rules();
    for r in &mut rules {
        r.compile()?;
    }

    let telegram = alert_config.and_then(|a| a.telegram);

    let state = Arc::new(ProxyState {
        client: Client::new(),
        target: config.target.trim_end_matches('/').to_string(),
        rules,
        mode: config.mode,
        telegram,
    });

    let app = Router::new()
        .route("/", any(proxy_handler))
        .route("/*path", any(proxy_handler))
        .with_state(state);

    let listener = TcpListener::bind(&config.listen).await?;
    info!("🛡️ MoltBot Harness proxy listening on {}", config.listen);
    info!("   Target: {}", config.target);
    info!("   Mode: {:?}", config.mode);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> impl IntoResponse {
    let path = uri.path();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let url = format!("{}{}{}", state.target, path, query);

    info!("📥 {} {} → {}", method, path, url);

    // Build upstream request
    let mut req_builder = match method {
        Method::GET => state.client.get(&url),
        Method::POST => state.client.post(&url),
        Method::PUT => state.client.put(&url),
        Method::DELETE => state.client.delete(&url),
        Method::PATCH => state.client.patch(&url),
        _ => state.client.get(&url),
    };

    // Forward headers (except host)
    for (name, value) in headers.iter() {
        if name == "host" {
            continue;
        }
        if let Ok(v) = value.to_str() {
            req_builder = req_builder.header(name.as_str(), v);
        }
    }

    // Forward body
    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Failed to read request body"))
                .unwrap();
        }
    };

    if !body_bytes.is_empty() {
        req_builder = req_builder.body(body_bytes.to_vec());
    }

    // Send upstream
    let upstream_resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Upstream request failed: {}", e);
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(format!("Upstream error: {}", e)))
                .unwrap();
        }
    };

    let status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();
    let is_api_post = method == Method::POST && (
        path.contains("/v1/messages") ||           // Anthropic
        path.contains("/v1/chat/completions") ||    // OpenAI-compatible
        path.contains("/generateContent")           // Gemini
    );
    let is_messages_post = is_api_post;
    let content_type = resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let is_streaming = content_type.contains("text/event-stream");

    // Streaming responses: intercept SSE events on the fly
    if is_messages_post && is_streaming {
        info!("📡 Streaming response detected — intercepting SSE events");
        let enforce = state.mode == ProxyMode::Enforce;
        let rules = state.rules.clone();
        let telegram = state.telegram.clone();

        let upstream_stream = upstream_resp.bytes_stream();

        let intercepted_stream = async_stream::stream! {
            let mut interceptor = StreamInterceptor::new(rules, enforce);
            let mut line_buf = SseLineBuffer::new();

            tokio::pin!(upstream_stream);

            while let Some(chunk_result) = upstream_stream.next().await {
                let chunk: bytes::Bytes = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Upstream stream error: {}", e);
                        break;
                    }
                };

                let text = match std::str::from_utf8(&chunk) {
                    Ok(t) => t.to_string(),
                    Err(_) => {
                        yield Ok::<bytes::Bytes, std::io::Error>(chunk);
                        continue;
                    }
                };

                let event_blocks = line_buf.feed(&text);
                for block in event_blocks {
                    let sse_events = parse_sse_events(&block);
                    for sse_event in sse_events {
                        let output_events = interceptor.process_event(sse_event);
                        for out in output_events {
                            yield Ok::<bytes::Bytes, std::io::Error>(bytes::Bytes::from(out.to_sse_bytes()));
                        }
                    }
                }
            }

            // Send alerts for any intercepts
            if !interceptor.intercepts.is_empty() {
                let intercepts = interceptor.intercepts.clone();
                tokio::spawn(async move {
                    send_intercept_alerts(telegram, &intercepts).await;
                });
            }
        };

        let mut builder = Response::builder()
            .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

        for (name, value) in resp_headers.iter() {
            if name == "transfer-encoding" || name == "content-length" {
                continue;
            }
            if let Ok(v) = value.to_str() {
                builder = builder.header(name.as_str(), v);
            }
        }

        return builder
            .body(Body::from_stream(intercepted_stream))
            .unwrap();
    }

    // Non-streaming: read full body
    let resp_body = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read upstream response: {}", e);
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("Failed to read upstream response"))
                .unwrap();
        }
    };

    // Intercept /v1/messages POST non-streaming responses
    let final_body = if is_messages_post {
        let enforce = state.mode == ProxyMode::Enforce;
        let (modified, intercepts) = intercept_response(&resp_body, &state.rules, enforce);

        if !intercepts.is_empty() {
            let telegram = state.telegram.clone();
            let intercepts_clone = intercepts.clone();
            tokio::spawn(async move {
                send_intercept_alerts(telegram, &intercepts_clone).await;
            });
        }

        modified
    } else {
        resp_body.to_vec()
    };

    // Build response
    let mut builder = Response::builder().status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK));

    for (name, value) in resp_headers.iter() {
        if name == "transfer-encoding" || name == "content-length" {
            continue;
        }
        if let Ok(v) = value.to_str() {
            builder = builder.header(name.as_str(), v);
        }
    }

    builder = builder.header("content-length", final_body.len().to_string());

    builder.body(Body::from(final_body)).unwrap()
}

async fn send_intercept_alerts(telegram: Option<TelegramConfig>, intercepts: &[InterceptResult]) {
    let Some(tg) = telegram else { return };
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", tg.bot_token);

    for intercept in intercepts {
        if !matches!(intercept.action, RuleAction::CriticalAlert | RuleAction::PauseAndAsk) {
            continue;
        }
        let message = format_telegram_alert(intercept);
        if let Err(e) = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": tg.chat_id,
                "text": message,
                "parse_mode": "Markdown"
            }))
            .send()
            .await
        {
            error!("Failed to send Telegram alert: {}", e);
        }
    }
}
