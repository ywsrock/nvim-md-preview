//! 全リクエストの入口でのアクセス制御（Host / Origin の確認、LAN モードのトークン）

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use crate::state::AppState;

pub(crate) const TOKEN_COOKIE: &str = "mdp_token";

/// "host:port" / "[::1]:port" からホスト名部分を取り出す
pub(crate) fn host_name(host_header: &str) -> &str {
    if let Some(rest) = host_header.strip_prefix('[') {
        return rest.split(']').next().unwrap_or("");
    }
    host_header.rsplit_once(':').map(|(h, _)| h).unwrap_or(host_header)
}

pub(crate) fn is_loopback_name(name: &str) -> bool {
    matches!(name, "127.0.0.1" | "localhost" | "::1")
}

/// 全リクエストの入口でのアクセス制御
/// - localhost 公開時は Host が localhost 系であること（DNS rebinding 対策）
/// - Origin 付きのリクエスト（fetch / WebSocket）は同一オリジンであること（他サイトからの操作を拒否）
/// - トークンがあれば ?token= か Cookie で一致すること。?token= で来たら Cookie を発行する
pub(crate) async fn access_guard(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let headers = req.headers();
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();

    if !state.access.lan && !is_loopback_name(host_name(&host)) {
        return (StatusCode::FORBIDDEN, "forbidden host").into_response();
    }
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        let origin_host = origin.split_once("://").map(|(_, h)| h).unwrap_or("");
        if origin_host != host {
            return (StatusCode::FORBIDDEN, "cross-origin request").into_response();
        }
    }

    let mut set_cookie = None;
    if let Some(token) = &state.access.token {
        let from_query = req.uri().query().and_then(|q| {
            q.split('&').find_map(|kv| kv.strip_prefix("token=")).map(str::to_string)
        });
        let from_cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).and_then(|c| {
            c.split(';').find_map(|kv| kv.trim().strip_prefix(&format!("{}=", TOKEN_COOKIE)).map(str::to_string))
        });
        let ok = from_cookie.as_deref() == Some(token.as_str()) || from_query.as_deref() == Some(token.as_str());
        if !ok {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "A token is required. Open the URL with ?token= shown in Neovim.",
            ).into_response();
        }
        if from_query.as_deref() == Some(token.as_str()) {
            set_cookie = Some(format!("{}={}; Path=/; HttpOnly; SameSite=Strict", TOKEN_COOKIE, token));
        }
    }

    let mut res = next.run(req).await;
    if let Some(c) = set_cookie {
        if let Ok(v) = header::HeaderValue::from_str(&c) {
            res.headers_mut().insert(header::SET_COOKIE, v);
        }
    }
    res
}
