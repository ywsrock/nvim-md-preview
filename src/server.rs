//! HTTP / WebSocket サーバーの起動とルーティング、ブラウザとの WebSocket のやり取り

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use crate::access::access_guard;
use crate::assets::*;
use crate::export::export_handler;
use crate::page::*;
use crate::plantuml::plantuml_handler;
pub use crate::state::{Access, AppState, NvimNotify};
pub use crate::export::export_bytes;

pub async fn start_server(host: &str, port: u16, state: Arc<AppState>) {
    let listener = find_listener(host, port).await;
    let bound_port = listener.local_addr().unwrap().port();
    state.port.store(bound_port, std::sync::atomic::Ordering::Relaxed);

    let app = Router::new()
        .route("/", get(preview_page))
        .route("/ws", get(ws_handler))
        .route("/assets", get(assets_handler))
        .route("/export", get(export_handler))
        .route("/plantuml", post(plantuml_handler))
        .route("/app.css", get(|| async { static_text("text/css; charset=utf-8", APP_CSS) }))
        .route("/app.js", get(|| async { static_text("application/javascript; charset=utf-8", APP_JS) }))
        .route("/settings", get(settings_handler))
        .route("/print", get(print_page))
        .route("/custom.css", get(custom_css_handler))
        .route("/lib/highlight.min.js",    get(|| async { js_response(HLJS_JS) }))
        .route("/lib/hljs-dark.min.css",   get(|| async { css_response(HLJS_DARK_CSS) }))
        .route("/lib/hljs-light.min.css",  get(|| async { css_response(HLJS_LIGHT_CSS) }))
        .route("/lib/mermaid.min.js",      get(|| async { js_response(MERMAID_JS) }))
        .route("/lib/katex.min.js",        get(|| async { js_response(KATEX_JS) }))
        .route("/lib/katex.min.css",       get(|| async { css_response(KATEX_CSS) }))
        .route("/lib/auto-render.min.js",  get(|| async { js_response(AUTO_RENDER_JS) }))
        .route("/lib/fonts/:name",         get(katex_font_handler))
        .layer(axum::middleware::from_fn_with_state(Arc::clone(&state), access_guard))
        .with_state(state);

    eprintln!("READY:{}", bound_port);
    axum::serve(listener, app).await.unwrap();
}

/// 指定ポートから順に空きポートを探す
async fn find_listener(host: &str, start_port: u16) -> tokio::net::TcpListener {
    for port in start_port..start_port.saturating_add(10) {
        let addr = format!("{}:{}", host, port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => return l,
            Err(_) => continue,
        }
    }
    // 0 を指定するとOSが空きポートを自動割り当て
    tokio::net::TcpListener::bind(format!("{}:0", host))
        .await
        .expect("Failed to bind any port")
}

/// WebSocket アップグレードハンドラ
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    {
        let latest = state.latest_html.read().await;
        if let Some(payload) = latest.as_ref() {
            if socket.send(Message::Text(payload.clone())).await.is_err() {
                return;
            }
        }
        let diff = state.latest_diff.read().await;
        if let Some(payload) = diff.as_ref() {
            if socket.send(Message::Text(payload.clone())).await.is_err() {
                return;
            }
        }
    }
    let mut rx = state.tx.subscribe();
    loop {
        tokio::select! {
            // Neovim → ブラウザ（内容更新・スクロール）
            msg = rx.recv() => match msg {
                Ok(payload) => {
                    if socket.send(Message::Text(payload)).await.is_err() { break; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            // ブラウザ → Neovim（クリックで該当行へジャンプ）
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => handle_browser_message(&state, &text),
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
}

/// ブラウザから届いた JSON メッセージを処理する
/// {"type":"goto","line":N} → Neovim のカーソルを N 行目へ
/// {"type":"open","path":"other.md"} → Neovim でファイルを開く（Wiki リンク・相対リンク）
fn handle_browser_message(state: &AppState, text: &str) {
    let Ok(data) = serde_json::from_str::<serde_json::Value>(text) else { return; };
    match data.get("type").and_then(|v| v.as_str()) {
        Some("goto") => {
            let Some(line) = data.get("line").and_then(|v| v.as_u64()) else { return; };
            state.notify_nvim(
                "nvim_exec_lua",
                vec![
                    rmpv::Value::from("require('md_preview')._goto_line(...)"),
                    rmpv::Value::Array(vec![rmpv::Value::from(line)]),
                ],
            );
        }
        Some("open") => {
            let Some(path) = data.get("path").and_then(|v| v.as_str()) else { return; };
            state.notify_nvim(
                "nvim_exec_lua",
                vec![
                    rmpv::Value::from("require('md_preview')._open_link(...)"),
                    rmpv::Value::Array(vec![rmpv::Value::from(path)]),
                ],
            );
        }
        _ => {}
    }
}
