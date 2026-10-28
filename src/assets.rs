//! バイナリに埋め込んだ JS / CSS / フォントと、文書のディレクトリの画像の配信

// JS/CSS ライブラリを gzip 圧縮して埋め込む（ブラウザに Content-Encoding: gzip で返す）
pub(crate) static HLJS_JS:         &[u8] = include_bytes!("embedded_lib/highlight.min.js.gz");
pub(crate) static HLJS_DARK_CSS:   &[u8] = include_bytes!("embedded_lib/hljs-dark.min.css.gz");
pub(crate) static HLJS_LIGHT_CSS:  &[u8] = include_bytes!("embedded_lib/hljs-light.min.css.gz");
pub(crate) static MERMAID_JS:      &[u8] = include_bytes!("embedded_lib/mermaid.min.js.gz");
pub(crate) static KATEX_JS:        &[u8] = include_bytes!("embedded_lib/katex.min.js.gz");
pub(crate) static KATEX_CSS:       &[u8] = include_bytes!("embedded_lib/katex.min.css.gz");
pub(crate) static AUTO_RENDER_JS:  &[u8] = include_bytes!("embedded_lib/auto-render.min.js.gz");

// KaTeX フォント（woff2）を埋め込む
macro_rules! font {
    ($n:literal) => { ($n, include_bytes!(concat!("embedded_lib/fonts/", $n, ".woff2")) as &[u8]) };
}
pub(crate) static KATEX_FONTS: &[(&str, &[u8])] = &[
    font!("KaTeX_AMS-Regular"),
    font!("KaTeX_Caligraphic-Bold"),
    font!("KaTeX_Caligraphic-Regular"),
    font!("KaTeX_Fraktur-Bold"),
    font!("KaTeX_Fraktur-Regular"),
    font!("KaTeX_Main-Bold"),
    font!("KaTeX_Main-BoldItalic"),
    font!("KaTeX_Main-Italic"),
    font!("KaTeX_Main-Regular"),
    font!("KaTeX_Math-BoldItalic"),
    font!("KaTeX_Math-Italic"),
    font!("KaTeX_SansSerif-Bold"),
    font!("KaTeX_SansSerif-Italic"),
    font!("KaTeX_SansSerif-Regular"),
    font!("KaTeX_Script-Regular"),
    font!("KaTeX_Size1-Regular"),
    font!("KaTeX_Size2-Regular"),
    font!("KaTeX_Size3-Regular"),
    font!("KaTeX_Size4-Regular"),
    font!("KaTeX_Typewriter-Regular"),
];

use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::{path::Path, sync::Arc};
use crate::state::AppState;

pub(crate) fn js_response(data: &'static [u8]) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CONTENT_ENCODING, "gzip"),
        ],
        data,
    )
}
pub(crate) fn css_response(data: &'static [u8]) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CONTENT_ENCODING, "gzip"),
        ],
        data,
    )
}

/// KaTeX フォント（/lib/fonts/:name.woff2）
pub(crate) async fn katex_font_handler(AxumPath(name): AxumPath<String>) -> Response {
    for (font_name, data) in KATEX_FONTS {
        if format!("{}.woff2", font_name) == name {
            return ([(header::CONTENT_TYPE, "font/woff2")], *data).into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

/// /assets?p=relative/path — 相対パス画像を提供する
#[derive(Deserialize)]
pub(crate) struct AssetsParams { p: String }

pub(crate) async fn assets_handler(
    Query(params): Query<AssetsParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    // パストラバーサル防止を最初にチェック（base_dir の有無より先）
    // 絶対パス・ドライブ指定・".." を含むものは拒否する（Path::join は絶対パスで置き換わるため）
    let rel = Path::new(&params.p);
    if params.p.contains("..")
        || rel.components().any(|c| !matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir))
    {
        return StatusCode::FORBIDDEN.into_response();
    }

    let base_dir = state.base_dir.read().await;
    let Some(dir) = base_dir.as_ref() else { return StatusCode::NOT_FOUND.into_response(); };
    let path = dir.join(rel);
    // シンボリックリンクで外に出ていないか、実体のパスで確認する
    let (Ok(real), Ok(real_dir)) = (tokio::fs::canonicalize(&path).await, tokio::fs::canonicalize(dir).await) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !real.starts_with(&real_dir) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match tokio::fs::read(&real).await {
        Ok(data) => {
            let mime = mime_from_ext(&path);
            ([(header::CONTENT_TYPE, mime)], data).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// 拡張子から MIME タイプを返す
pub fn mime_from_ext(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png")  => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif")  => "image/gif",
        Some("svg")  => "image/svg+xml",
        Some("webp") => "image/webp",
        _            => "application/octet-stream",
    }
}
