//! プレビュー画面（src/preview/ の HTML・CSS・JS）、PDF 用の印刷ページ、ブラウザ向けの設定

use axum::{
    extract::{RawQuery, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use std::sync::Arc;
use crate::state::AppState;

/// プレビュー画面。本体の CSS と JS は /app.css・/app.js で別に配信する
pub(crate) const PREVIEW_HTML: &str = include_str!("preview/index.html");
pub(crate) const APP_CSS: &str = include_str!("preview/style.css");
pub(crate) const APP_JS: &str = include_str!("preview/app.js");

/// /app.css・/app.js を返す（バイナリを更新したら必ず新しいものを読ませるため、キャッシュさせない）
pub(crate) fn static_text(content_type: &'static str, body: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type), (header::CACHE_CONTROL, "no-store")], body).into_response()
}

/// プレビューページの HTML を返す
/// URL の ?instance= がこのサーバーの ID と違えば、別の Neovim のプレビューにつながっているので内容を出さない
pub(crate) async fn preview_page(State(state): State<Arc<AppState>>, RawQuery(query): RawQuery) -> Response {
    let wanted = query.as_deref().and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("instance=")));
    if let (Some(wanted), Some(own)) = (wanted, state.access.instance.as_deref()) {
        if wanted != own {
            return (StatusCode::CONFLICT, Html(OTHER_PREVIEW_HTML)).into_response();
        }
    }
    Html(PREVIEW_HTML).into_response()
}

/// 別のプレビューのサーバーにつながったときのページ
const OTHER_PREVIEW_HTML: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Another preview is using this port</title>
<style>body{font:15px/1.6 -apple-system,"Segoe UI",sans-serif;max-width:40rem;margin:4rem auto;padding:0 1rem;color:#24292f}
code{background:#f0f0f0;padding:.1em .3em;border-radius:4px}</style></head>
<body><h1>Another preview is using this port</h1>
<p>This address is served by a preview started from another Neovim, so the document you opened is not shown here.</p>
<p>This can happen when Neovim on Windows and Neovim in WSL both run a preview. Stop the other preview and run <code>:MdPreview</code> again, or set a different <code>port</code> in <code>setup()</code>.</p>
</body></html>"#;

/// /print — PDF 用のページ。最新の内容と出力の設定を埋め込み、WebSocket を待たずに描画する
pub(crate) async fn print_page(State(state): State<Arc<AppState>>) -> Response {
    let content = state.latest_html.read().await.clone().unwrap_or_else(|| "null".to_string());
    let opts = state.export_opts.read().await.clone();
    let options = serde_json::json!({
        "paper": css_paper(&opts.paper),
        "margin": opts.margin.parse::<f64>().map(|n| format!("{}cm", n)).unwrap_or_else(|_| opts.margin.clone()),
        "font": opts.font,
        "font_size": opts.font_size,
        "toc": opts.toc,
        "numbering": opts.numbering,
    });
    // 文書の中の "</script>" でスクリプトが途中で閉じないようにする
    const APP_SCRIPT: &str = r#"<script type="module" src="/app.js"></script>"#;
    let script = format!(
        "<script>window.__MDP_PRINT__ = {{\"content\": {}, \"options\": {}}};</script>\n  {}",
        content.replace("</", "<\\/"),
        options.to_string().replace("</", "<\\/"),
        APP_SCRIPT,
    );
    Html(PREVIEW_HTML.replacen(APP_SCRIPT, &script, 1)).into_response()
}

/// Typst の用紙名を CSS の @page size に直す（"us-letter" → "letter" など）
pub(crate) fn css_paper(paper: &str) -> String {
    match paper.to_ascii_lowercase().as_str() {
        "us-letter" => "letter".to_string(),
        "us-legal" => "legal".to_string(),
        "us-tabloid" => "ledger".to_string(),
        p if p.len() <= 3 && p.starts_with(['a', 'b', 'c']) => p.to_uppercase(),
        p => p.to_string(),
    }
}

/// /settings — ブラウザの初期設定（テーマなど）
pub(crate) async fn settings_handler(State(state): State<Arc<AppState>>) -> Response {
    let theme = state.theme.read().await.clone();
    let browser_keys = *state.browser_keys.read().await;
    let task_toggle = *state.task_toggle.read().await && !state.access.lan;
    let cursor_highlight = *state.cursor_highlight.read().await;
    let export_formats = state.export_formats.read().await.clone();
    (
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({
            "theme": theme,
            "browser_keys": browser_keys,
            "cursor_highlight": cursor_highlight,
            "task_toggle": task_toggle,
            "export_formats": export_formats,
        }).to_string(),
    ).into_response()
}

/// /custom.css — ユーザー CSS（未設定なら空）
pub(crate) async fn custom_css_handler(State(state): State<Arc<AppState>>) -> Response {
    let css = state.custom_css.read().await.clone();
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8"), (header::CACHE_CONTROL, "no-store")],
        css,
    ).into_response()
}
