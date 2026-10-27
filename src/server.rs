// JS/CSS ライブラリを gzip 圧縮して埋め込む（ブラウザに Content-Encoding: gzip で返す）
static HLJS_JS:         &[u8] = include_bytes!("embedded_lib/highlight.min.js.gz");
static HLJS_DARK_CSS:   &[u8] = include_bytes!("embedded_lib/hljs-dark.min.css.gz");
static HLJS_LIGHT_CSS:  &[u8] = include_bytes!("embedded_lib/hljs-light.min.css.gz");
static MERMAID_JS:      &[u8] = include_bytes!("embedded_lib/mermaid.min.js.gz");
static KATEX_JS:        &[u8] = include_bytes!("embedded_lib/katex.min.js.gz");
static KATEX_CSS:       &[u8] = include_bytes!("embedded_lib/katex.min.css.gz");
static AUTO_RENDER_JS:  &[u8] = include_bytes!("embedded_lib/auto-render.min.js.gz");

// KaTeX フォント（woff2）を埋め込む
macro_rules! font {
    ($n:literal) => { ($n, include_bytes!(concat!("embedded_lib/fonts/", $n, ".woff2")) as &[u8]) };
}
static KATEX_FONTS: &[(&str, &[u8])] = &[
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
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path as AxumPath, Query, State,
    },
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{path::{Path, PathBuf}, sync::Arc};
use tokio::sync::{broadcast, mpsc, RwLock};

pub type HtmlSender = broadcast::Sender<String>;
/// Neovim へ送る通知 (method, params)
pub type NvimNotify = (String, Vec<rmpv::Value>);

#[derive(Clone)]
pub struct AppState {
    pub tx: HtmlSender,
    pub latest_html: Arc<RwLock<Option<String>>>,
    pub base_dir: Arc<RwLock<Option<PathBuf>>>,
    /// 現在プレビュー中のファイルのフルパス（エクスポートに使用）
    pub current_file: Arc<RwLock<Option<PathBuf>>>,
    /// pandoc バイナリのパス
    pub pandoc_path: Arc<RwLock<Option<PathBuf>>>,
    /// Typst バイナリのパス
    pub typst_path: Arc<RwLock<Option<PathBuf>>>,
    /// 現在のバッファ内容（未保存分も含む。エクスポートに使用）
    pub current_content: Arc<RwLock<Option<String>>>,
    /// Neovim への RPC 通知チャネル（stdout に書く）
    pub nvim_tx: mpsc::UnboundedSender<NvimNotify>,
}

impl AppState {
    pub fn new(tx: HtmlSender, nvim_tx: mpsc::UnboundedSender<NvimNotify>) -> Self {
        Self {
            tx,
            latest_html: Arc::new(RwLock::new(None)),
            base_dir: Arc::new(RwLock::new(None)),
            current_file: Arc::new(RwLock::new(None)),
            pandoc_path: Arc::new(RwLock::new(None)),
            typst_path: Arc::new(RwLock::new(None)),
            current_content: Arc::new(RwLock::new(None)),
            nvim_tx,
        }
    }

    /// Neovim の API を通知として呼ぶ（応答は受け取らない）
    pub fn notify_nvim(&self, method: &str, params: Vec<rmpv::Value>) {
        let _ = self.nvim_tx.send((method.to_string(), params));
    }

    pub async fn update_html(&self, payload: String) {
        *self.latest_html.write().await = Some(payload.clone());
        let _ = self.tx.send(payload);
    }

    pub fn send_scroll(&self, payload: String) {
        let _ = self.tx.send(payload);
    }
}

pub async fn start_server(port: u16, state: Arc<AppState>) {
    let listener = find_listener(port).await;
    let bound_port = listener.local_addr().unwrap().port();

    let app = Router::new()
        .route("/", get(preview_page))
        .route("/ws", get(ws_handler))
        .route("/assets", get(assets_handler))
        .route("/export", get(export_handler))
        .route("/lib/highlight.min.js",    get(|| async { js_response(HLJS_JS) }))
        .route("/lib/hljs-dark.min.css",   get(|| async { css_response(HLJS_DARK_CSS) }))
        .route("/lib/hljs-light.min.css",  get(|| async { css_response(HLJS_LIGHT_CSS) }))
        .route("/lib/mermaid.min.js",      get(|| async { js_response(MERMAID_JS) }))
        .route("/lib/katex.min.js",        get(|| async { js_response(KATEX_JS) }))
        .route("/lib/katex.min.css",       get(|| async { css_response(KATEX_CSS) }))
        .route("/lib/auto-render.min.js",  get(|| async { js_response(AUTO_RENDER_JS) }))
        .route("/lib/fonts/:name",         get(katex_font_handler))
        .layer(axum::middleware::from_fn(access_guard))
        .with_state(state);

    eprintln!("READY:{}", bound_port);
    axum::serve(listener, app).await.unwrap();
}

/// 指定ポートから順に空きポートを探す
async fn find_listener(start_port: u16) -> tokio::net::TcpListener {
    for port in start_port..start_port.saturating_add(10) {
        let addr = format!("127.0.0.1:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => return l,
            Err(_) => continue,
        }
    }
    // 0 を指定するとOSが空きポートを自動割り当て
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind any port")
}

/// "host:port" / "[::1]:port" からホスト名部分を取り出す
fn host_name(host_header: &str) -> &str {
    if let Some(rest) = host_header.strip_prefix('[') {
        return rest.split(']').next().unwrap_or("");
    }
    host_header.rsplit_once(':').map(|(h, _)| h).unwrap_or(host_header)
}

/// 全リクエストの入口でのアクセス制御
/// - Host が localhost 系であること（DNS rebinding 対策）
/// - Origin 付きのリクエスト（fetch / WebSocket）は同一オリジンであること（他サイトからの読み取りを拒否）
async fn access_guard(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let headers = req.headers();
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
    if !matches!(host_name(host), "127.0.0.1" | "localhost" | "::1") {
        return (StatusCode::FORBIDDEN, "forbidden host").into_response();
    }
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        let origin_host = origin.split_once("://").map(|(_, h)| h).unwrap_or("");
        if origin_host != host {
            return (StatusCode::FORBIDDEN, "cross-origin request").into_response();
        }
    }
    next.run(req).await
}

fn js_response(data: &'static [u8]) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CONTENT_ENCODING, "gzip"),
        ],
        data,
    )
}
fn css_response(data: &'static [u8]) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CONTENT_ENCODING, "gzip"),
        ],
        data,
    )
}

/// KaTeX フォント（/lib/fonts/:name.woff2）
async fn katex_font_handler(AxumPath(name): AxumPath<String>) -> Response {
    for (font_name, data) in KATEX_FONTS {
        if format!("{}.woff2", font_name) == name {
            return ([(header::CONTENT_TYPE, "font/woff2")], *data).into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

/// プレビューページの HTML を返す
async fn preview_page() -> impl IntoResponse {
    Html(PREVIEW_HTML)
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
fn handle_browser_message(state: &AppState, text: &str) {
    let Ok(data) = serde_json::from_str::<serde_json::Value>(text) else { return; };
    if data.get("type").and_then(|v| v.as_str()) != Some("goto") { return; }
    let Some(line) = data.get("line").and_then(|v| v.as_u64()) else { return; };
    state.notify_nvim(
        "nvim_exec_lua",
        vec![
            rmpv::Value::from("require('md_preview')._goto_line(...)"),
            rmpv::Value::Array(vec![rmpv::Value::from(line)]),
        ],
    );
}

/// /assets?p=relative/path — 相対パス画像を提供する
#[derive(Deserialize)]
struct AssetsParams { p: String }

async fn assets_handler(
    Query(params): Query<AssetsParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    // パストラバーサル防止を最初にチェック（base_dir の有無より先）
    // 絶対パス・ドライブ指定・".." を含むものは拒否する（Path::join は絶対パスで置き換わるため）
    let rel = std::path::Path::new(&params.p);
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

/// /export?format=pdf|docx|html|epub|markdown — エクスポートハンドラ
#[derive(Deserialize)]
struct ExportParams {
    format: String,
}

async fn export_handler(
    Query(params): Query<ExportParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    match export_bytes(&state, &params.format.to_lowercase()).await {
        Ok((data, filename)) => download_response(data, &filename, mime_for_format(&filename)),
        Err(e) => error_response(&e),
    }
}

/// 形式名 → 拡張子（"md" / "markdown" はどちらも .md）
fn extension_for(format: &str) -> Option<&'static str> {
    match format {
        "markdown" | "md" => Some("md"),
        "docx" => Some("docx"),
        "html" => Some("html"),
        "epub" => Some("epub"),
        "pdf"  => Some("pdf"),
        _ => None,
    }
}

fn mime_for_format(filename: &str) -> &'static str {
    match filename.rsplit('.').next().unwrap_or("") {
        "md"   => "text/markdown; charset=utf-8",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "html" => "text/html; charset=utf-8",
        "epub" => "application/epub+zip",
        "pdf"  => "application/pdf",
        _      => "application/octet-stream",
    }
}

/// 現在のバッファ内容を指定形式に変換して (バイト列, ファイル名) を返す
/// HTTP (/export) と RPC (:MdPreviewExport) の両方から使う
pub async fn export_bytes(state: &AppState, format: &str) -> Result<(Vec<u8>, String), String> {
    let Some(ext) = extension_for(format) else {
        return Err(format!("未対応の形式: {}", format));
    };

    // プレビュー中の内容（未保存分も含む）を使う
    let content = state.current_content.read().await.clone()
        .ok_or_else(|| "ファイルが開かれていません。:MdPreview でプレビューを開始してください。".to_string())?;
    let current_file = state.current_file.read().await.clone();
    let base_dir = state.base_dir.read().await.clone();
    let stem = current_file.as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("export")
        .to_string();
    let filename = format!("{}.{}", stem, ext);

    if ext == "md" {
        return Ok((content.into_bytes(), filename));
    }

    // 変換ツールにはファイルで渡す。同時実行で衝突しないよう一意な名前にする
    let work = std::env::temp_dir().join(format!("nvim-md-preview-{}-{}", std::process::id(), unique_id()));
    tokio::fs::create_dir_all(&work).await.map_err(|e| format!("一時ディレクトリの作成に失敗: {}", e))?;
    let md_tmp = work.join("input.md");
    tokio::fs::write(&md_tmp, &content).await.map_err(|e| format!("一時ファイルの作成に失敗: {}", e))?;

    let result = if ext == "pdf" {
        let typst = state.typst_path.read().await;
        let typst_bin = typst.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "typst".to_string());
        convert_with_typst(&typst_bin, &work).await
    } else {
        let pandoc = state.pandoc_path.read().await;
        let pandoc_bin = pandoc.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "pandoc".to_string());
        convert_with_pandoc(&pandoc_bin, &work, ext, base_dir.as_ref()).await
    };

    let _ = tokio::fs::remove_dir_all(&work).await;
    result.map(|data| (data, filename))
}

fn unique_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// pandoc で work/input.md を変換して出力バイト列を返す
async fn convert_with_pandoc(
    pandoc_bin: &str,
    work: &Path,
    format: &str,
    base_dir: Option<&PathBuf>,
) -> Result<Vec<u8>, String> {
    let in_path  = work.join("input.md");
    let out_path = work.join(format!("output.{}", format));

    let mut args = vec![
        in_path.to_string_lossy().to_string(),
        "-o".to_string(),
        out_path.to_string_lossy().to_string(),
        "--from=markdown+gfm_auto_identifiers".to_string(),
    ];
    // 相対パスの画像はプレビュー中ファイルのディレクトリから探す
    if let Some(dir) = base_dir {
        args.push(format!("--resource-path={}", dir.to_string_lossy()));
    }
    // HTML はスタンドアロンで出力
    if format == "html" {
        args.push("--standalone".to_string());
    }

    let output = tokio::process::Command::new(pandoc_bin)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("pandoc が見つかりません。:MdPreviewInstall でインストールしてください。\n\nエラー: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "pandoc 変換に失敗しました。pandoc がインストールされているか確認してください。\n\
             インストール: :MdPreviewInstall\n\nエラー: {}",
            truncate_chars(&stderr, 500)
        ));
    }
    tokio::fs::read(&out_path).await.map_err(|_| "変換結果の読み込みに失敗しました。".to_string())
}

/// Typst で work/input.md を PDF に変換して出力バイト列を返す
async fn convert_with_typst(typst_bin: &str, work: &Path) -> Result<Vec<u8>, String> {
    // Typst の cmarker パッケージで Markdown をそのまま処理
    // Markdown は raw ブロックに埋め込むと ``` を含む文書で壊れるので、ファイルを read() で読む
    let typ_content = r#"#import "@preview/cmarker:0.1.4": render
#set page(margin: 2cm)
#set text(font: ("Noto Serif CJK JP", "Noto Serif", "Georgia"), size: 11pt)
#set heading(numbering: none)

#render(read("input.md"), h1-level: 1)
"#;
    let typ_path = work.join("input.typ");
    let pdf_path = work.join("output.pdf");
    tokio::fs::write(&typ_path, typ_content).await.map_err(|e| format!("一時ファイルの作成に失敗: {}", e))?;

    let output = tokio::process::Command::new(typst_bin)
        .args(["compile", &typ_path.to_string_lossy(), &pdf_path.to_string_lossy()])
        .output()
        .await
        .map_err(|e| format!("Typst が見つかりません。:MdPreviewInstall でインストールしてください。\n\nエラー: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Typst 変換に失敗しました。:MdPreviewInstall で Typst をインストールしてください。\n\nエラー: {}",
            truncate_chars(&stderr, 500)
        ));
    }
    tokio::fs::read(&pdf_path).await.map_err(|_| "PDF の読み込みに失敗しました。".to_string())
}

/// 文字境界を壊さずに先頭 max 文字を返す（バイト単位スライスは多バイト文字で panic する）
fn truncate_chars(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

/// Content-Disposition を組み立てる
/// ヘッダー値は ASCII のみ許されるので、filename は ASCII に置き換え、
/// 元の名前は RFC 5987 の filename* にパーセントエンコードして入れる
fn content_disposition(filename: &str) -> String {
    let ascii: String = filename
        .chars()
        .map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' { c } else { '_' })
        .collect();
    let encoded: String = filename
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect();
    format!("attachment; filename=\"{}\"; filename*=UTF-8''{}", ascii, encoded)
}

/// ダウンロードレスポンスを生成
fn download_response(data: Vec<u8>, filename: &str, mime: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (header::CONTENT_DISPOSITION, content_disposition(filename)),
        ],
        data,
    ).into_response()
}

/// エラーレスポンスを JSON で返す
fn error_response(msg: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({"error": msg}).to_string(),
    ).into_response()
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

const PREVIEW_HTML: &str = r#"<!DOCTYPE html>
<html lang="ja" data-theme="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>md Preview</title>
  <link id="hljs-theme" rel="stylesheet" href="/lib/hljs-dark.min.css">
  <link rel="stylesheet" href="/lib/katex.min.css">
  <style>
    :root {
      --bg:#0d1117;--panel:#161b22;--stroke:#30363d;--fg:#e6edf3;--fg-muted:#8b949e;
      --link:#58a6ff;--code-bg:#1b2028;--blockquote-border:#3b434b;
      --table-border:#30363d;--table-row-alt:rgba(255,255,255,.03);
      --err-bg:rgba(239,68,68,.12);--err-border:rgba(239,68,68,.35);--err-fg:#fecaca;
      --scroll-thumb:rgba(148,163,184,.35);--accent:#58a6ff;
      --badge-bg:rgba(255,255,255,.06);--badge-fg:#8b949e;
      --skeleton:rgba(255,255,255,.04);--skeleton-shine:rgba(255,255,255,.08);
      --color-h1:#58a6ff;--color-h2:#a371f7;--color-h3:#3fb950;
      --color-h4:#39d2c0;--color-h5:#d29922;--color-h6:#8b949e;
    }
    [data-theme="light"] {
      --bg:#fff;--panel:#f6f8fa;--stroke:#d0d7de;--fg:#1f2328;--fg-muted:#656d76;
      --link:#0969da;--code-bg:#f6f8fa;--blockquote-border:#d0d7de;
      --table-border:#d0d7de;--table-row-alt:rgba(0,0,0,.02);
      --err-bg:rgba(239,68,68,.08);--err-border:#fecaca;--err-fg:#7f1d1d;
      --scroll-thumb:rgba(100,116,139,.4);--accent:#0969da;
      --badge-bg:rgba(0,0,0,.04);--badge-fg:#656d76;
      --skeleton:rgba(0,0,0,.04);--skeleton-shine:rgba(0,0,0,.07);
      --color-h1:#0969da;--color-h2:#8250df;--color-h3:#1a7f37;
      --color-h4:#0e8a7e;--color-h5:#9a6700;--color-h6:#656d76;
    }
    *,*::before,*::after{box-sizing:border-box}
    html{scrollbar-width:thin;scrollbar-color:var(--scroll-thumb) transparent}
    html,body{height:100%;margin:0;background:var(--bg);color:var(--fg);
      font-family:-apple-system,BlinkMacSystemFont,"Segoe UI","Noto Sans",Helvetica,Arial,sans-serif;
      line-height:1.6;-webkit-font-smoothing:antialiased}
    header{position:sticky;top:0;z-index:10;display:flex;align-items:center;gap:.6rem;
      padding:.55rem 1.2rem;background:var(--panel);border-bottom:1px solid var(--stroke);
      backdrop-filter:blur(12px);-webkit-backdrop-filter:blur(12px)}
    header h1{margin:0;font-size:.9rem;font-weight:600;letter-spacing:.2px;flex:1;
      overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
    .spacer{flex:1}
    .src-line{display:block;height:0;margin:0;padding:0;overflow:hidden}
    #content.click-sync{cursor:default}
    .status-dot{width:7px;height:7px;border-radius:50%;background:var(--fg-muted);
      opacity:.4;transition:all .3s;flex-shrink:0}
    .status-dot.connected{background:#3fb950;opacity:1;box-shadow:0 0 6px rgba(63,185,80,.5)}
    .status-dot.error{background:#f85149;opacity:1}
    .btn{background:transparent;border:1px solid var(--stroke);color:var(--fg-muted);
      padding:.35rem;border-radius:8px;cursor:pointer;font-size:.82rem;font-weight:500;
      transition:border-color .15s,background .15s,color .15s;
      display:flex;align-items:center;justify-content:center;width:32px;height:32px}
    .btn:hover{background:rgba(127,127,127,.1);color:var(--fg)}
    .btn-text{width:auto;padding:.35rem .65rem}
    /* skeleton */
    .skeleton{padding:2rem;max-width:900px;margin:0 auto}
    .skeleton-line{height:14px;border-radius:6px;background:var(--skeleton);
      margin-bottom:12px;position:relative;overflow:hidden}
    .skeleton-line::after{content:'';position:absolute;inset:0;
      background:linear-gradient(90deg,transparent,var(--skeleton-shine),transparent);
      animation:shimmer 1.5s infinite}
    .skeleton-line:nth-child(1){width:45%;height:28px;margin-bottom:20px}
    .skeleton-line:nth-child(2){width:90%}.skeleton-line:nth-child(3){width:75%}
    .skeleton-line:nth-child(4){width:85%}.skeleton-line:nth-child(5){width:60%}
    .skeleton-line:nth-child(6){width:0;height:8px}
    .skeleton-line:nth-child(7){width:35%;height:22px;margin-bottom:16px}
    .skeleton-line:nth-child(8){width:80%}.skeleton-line:nth-child(9){width:70%}
    @keyframes shimmer{0%{transform:translateX(-100%)}100%{transform:translateX(100%)}}
    /* content */
    #content{max-width:900px;margin:0 auto;padding:2rem 2rem 4rem;animation:fadeIn .2s ease}
    @keyframes fadeIn{from{opacity:0;transform:translateY(4px)}to{opacity:1;transform:none}}
    #content>*:first-child{margin-top:0}
    #content h1,#content h2,#content h3,#content h4,#content h5,#content h6{
      position:relative;scroll-margin-top:3.2rem}
    #content h1{font-size:2em;border-bottom:1px solid var(--stroke);
      border-left:3px solid var(--color-h1);padding-bottom:.3em;padding-left:.6em;margin:1.5em 0 .5em}
    #content h2{font-size:1.5em;border-bottom:1px solid var(--stroke);
      border-left:3px solid var(--color-h2);padding-bottom:.25em;padding-left:.6em;margin:1.4em 0 .5em}
    #content h3{font-size:1.25em;border-left:3px solid var(--color-h3);padding-left:.6em;margin:1.3em 0 .4em}
    #content h4{font-size:1em;border-left:3px solid var(--color-h4);padding-left:.6em;margin:1.2em 0 .4em}
    #content h5,#content h6{font-size:.875em;color:var(--fg-muted);padding-left:.6em;margin:1em 0 .4em}
    #content h5{border-left:3px solid var(--color-h5)}
    #content h6{border-left:3px solid var(--color-h6)}
    #content p{margin:0 0 1em}
    #content a{color:var(--link);text-decoration:none}
    #content a:hover{text-decoration:underline}
    #content strong{font-weight:600}
    #content img{max-width:100%;border-radius:6px}
    #content hr{height:3px;background:var(--stroke);border:none;margin:1.5em 0}
    #content ul,#content ol{padding-left:2em;margin:0 0 1em}
    #content li+li{margin-top:.25em}
    #content .task-list-item{list-style:none;margin-left:-1.5em}
    #content .task-list-item input[type="checkbox"]{margin-right:.4em}
    #content blockquote{margin:0 0 1em;padding:.5em 1em;
      border-left:4px solid var(--blockquote-border);color:var(--fg-muted)}
    #content blockquote>*:last-child{margin-bottom:0}
    #content table{width:100%;border-collapse:collapse;margin:0 0 1em;font-size:.9em}
    #content th,#content td{padding:.5em .8em;border:1px solid var(--table-border)}
    #content th{background:var(--panel);font-weight:600}
    #content tr:nth-child(even){background:var(--table-row-alt)}
    #content code{background:var(--code-bg);padding:.15em .35em;border-radius:4px;
      font-size:.88em;font-family:ui-monospace,SFMono-Regular,"SF Mono",Menlo,Consolas,monospace}
    #content pre{position:relative;background:var(--code-bg);border:1px solid var(--stroke);
      border-radius:8px;padding:1em;overflow-x:auto;margin:0 0 1em;
      scrollbar-width:thin;scrollbar-color:var(--scroll-thumb) transparent}
    #content pre code{background:none;padding:0;border-radius:0;font-size:.85em;line-height:1.55}
    /* code wrap with lang badge + copy btn */
    #content .code-wrap{position:relative;margin:0 0 1em}
    #content .code-wrap pre{margin:0}
    #content .code-wrap .code-lang{position:absolute;top:0;right:0;padding:.2em .6em;
      font-size:.68rem;color:var(--badge-fg);background:var(--badge-bg);
      border-bottom-left-radius:6px;border-top-right-radius:7px;
      text-transform:uppercase;letter-spacing:.5px;font-weight:500;line-height:1.4;pointer-events:none;z-index:2}
    #content .code-wrap .copy-btn{position:absolute;top:6px;right:6px;width:28px;height:28px;
      display:flex;align-items:center;justify-content:center;
      background:var(--panel);border:1px solid var(--stroke);border-radius:6px;
      cursor:pointer;opacity:0;transition:opacity .15s,background .15s;color:var(--fg-muted);padding:0;z-index:2}
    #content .code-wrap:hover .copy-btn{opacity:1}
    #content .code-wrap .copy-btn:hover{color:var(--fg);background:rgba(127,127,127,.15)}
    #content .code-wrap .copy-btn.copied{color:#3fb950}
    #content .code-wrap .code-lang ~ .copy-btn{top:26px}
    /* mermaid */
    .mermaid-block{position:relative;margin:1em 0;border:1px solid var(--stroke);
      border-radius:10px;background:var(--panel);overflow:hidden;min-height:60px}
    .mermaid-block .mermaid-svg-wrap{padding:1rem;display:flex;justify-content:center;overflow:auto}
    .mermaid-block .mermaid-svg-wrap svg{max-width:100%;height:auto}
    .mermaid-block .mermaid-expand-btn{position:absolute;top:8px;right:8px;z-index:2;
      width:32px;height:32px;display:flex;align-items:center;justify-content:center;
      background:var(--panel);border:1px solid var(--stroke);border-radius:6px;
      cursor:pointer;opacity:0;transition:opacity .2s;color:var(--fg-muted)}
    .mermaid-block:hover .mermaid-expand-btn{opacity:1}
    .mermaid-block .mermaid-expand-btn:hover{color:var(--fg);background:rgba(127,127,127,.1)}
    .mermaid-block .mermaid-error{padding:.6rem 1rem;font-size:.85rem;color:var(--err-fg);
      background:var(--err-bg);border:1px solid var(--err-border);border-radius:6px;margin:.5rem}
    .mermaid-block :where(.error,.error-icon,.error-text,.errorText){display:none !important}
    /* overlay */
    #overlay{display:none;position:fixed;inset:0;z-index:100;background:var(--bg);flex-direction:column}
    #overlay.active{display:flex}
    #overlay-toolbar{display:flex;gap:.4rem;align-items:center;padding:.5rem 1rem;
      background:var(--panel);border-bottom:1px solid var(--stroke);flex-wrap:wrap}
    #overlay-toolbar .sep{width:1px;height:24px;background:var(--stroke);margin:0 .2rem}
    #overlay-toolbar .grow{flex:1}
    #overlay-panzoom{flex:1;overflow:auto;cursor:grab;
      scrollbar-width:thin;scrollbar-color:var(--scroll-thumb) transparent}
    #overlay-panzoom-inner{min-width:100%;min-height:100%;box-sizing:border-box;padding:1rem;
      display:flex;justify-content:center;align-items:flex-start;
      width:max-content;height:max-content}
    #overlay-panzoom.dragging{cursor:grabbing}
    #overlay-panzoom-inner svg{display:block}
    @media(max-width:768px){#content{padding:1rem 1rem 3rem}}
    /* Export メニュー */
    .export-item{display:block;width:100%;text-align:left;padding:.5rem .9rem;
      background:transparent;border:none;color:var(--fg);cursor:pointer;
      font-size:.82rem;white-space:nowrap;transition:background .1s}
    .export-item:hover{background:rgba(127,127,127,.12)}
    /* front matter panel */
    #fm-panel{margin:.5rem auto 0;max-width:900px;padding:0 2rem;
      border:none;font-size:.85rem}
    #fm-panel summary{padding:.45em .9em;cursor:pointer;color:var(--fg-muted);
      user-select:none;background:var(--panel);border:1px solid var(--stroke);
      border-radius:8px;list-style:none}
    #fm-panel[open] summary{border-radius:8px 8px 0 0}
    #fm-panel pre{margin:0;padding:.6em .9em;border:1px solid var(--stroke);
      border-top:none;border-radius:0 0 8px 8px;white-space:pre-wrap;
      color:var(--fg-muted);font-size:.82rem;background:var(--panel)}
    /* GitHub Alerts */
    #content .markdown-alert{padding:.5rem 1em;margin-bottom:16px;
      border-left:.25em solid #888;border-radius:0 6px 6px 0;background:var(--badge-bg)}
    #content .markdown-alert>:first-child{margin-top:0}
    #content .markdown-alert>:last-child{margin-bottom:0}
    #content .markdown-alert-title{display:flex;font-weight:500;align-items:center;
      line-height:1;margin-bottom:.5em;gap:.4em}
    #content .markdown-alert-note  {border-left-color:var(--color-note)}
    #content .markdown-alert-note   .markdown-alert-title{color:var(--color-note)}
    #content .markdown-alert-tip   {border-left-color:var(--color-tip)}
    #content .markdown-alert-tip    .markdown-alert-title{color:var(--color-tip)}
    #content .markdown-alert-warning{border-left-color:var(--color-warning)}
    #content .markdown-alert-warning .markdown-alert-title{color:var(--color-warning)}
    #content .markdown-alert-caution{border-left-color:var(--color-caution)}
    #content .markdown-alert-caution .markdown-alert-title{color:var(--color-caution)}
    #content .markdown-alert-important{border-left-color:var(--color-important)}
    #content .markdown-alert-important .markdown-alert-title{color:var(--color-important)}
    /* 見出しアンカーリンク */
    #content .anchor-link{opacity:0;margin-left:.4em;font-size:.8em;
      color:var(--fg-muted);text-decoration:none;transition:opacity .15s}
    #content h1:hover .anchor-link,#content h2:hover .anchor-link,
    #content h3:hover .anchor-link,#content h4:hover .anchor-link,
    #content h5:hover .anchor-link,#content h6:hover .anchor-link{opacity:1}
    /* KaTeX */
    .katex-display{overflow-x:auto;overflow-y:hidden;padding:.5em 0}
  </style>
</head>
<body>
  <header>
    <span style="display:flex;align-items:center;color:var(--fg-muted)">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <rect x="2" y="4" width="20" height="16" rx="2" stroke="currentColor" stroke-width="1.5"/>
        <path d="M7 15V9l2.5 3L12 9v6M16 12l2-3 2 3M16 12h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </span>
    <h1 id="title">md Preview</h1>
    <div id="statusDot" class="status-dot" title="Disconnected"></div>
    <button id="themeBtn" class="btn" title="Toggle light/dark">
      <svg id="themeIcon" width="16" height="16" viewBox="0 0 24 24" fill="none">
        <path id="moonPath" d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
    <!-- Export ドロップダウン -->
    <div style="position:relative">
      <button id="exportBtn" class="btn btn-text" title="エクスポート" style="font-size:.78rem;gap:.3rem;width:auto;padding:.35rem .65rem">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        Export
      </button>
      <div id="exportMenu" style="display:none;position:absolute;right:0;top:100%;min-width:160px;background:var(--panel);border:1px solid var(--stroke);border-radius:8px;box-shadow:0 4px 12px rgba(0,0,0,.3);z-index:20;overflow:hidden">
        <button class="export-item" data-format="pdf">📄 PDF <span style="font-size:.7rem;color:var(--fg-muted)">(Typst)</span></button>
        <button class="export-item" data-format="docx">📝 DOCX <span style="font-size:.7rem;color:var(--fg-muted)">(pandoc)</span></button>
        <button class="export-item" data-format="html">🌐 HTML <span style="font-size:.7rem;color:var(--fg-muted)">(pandoc)</span></button>
        <button class="export-item" data-format="epub">📚 EPUB <span style="font-size:.7rem;color:var(--fg-muted)">(pandoc)</span></button>
        <div style="height:1px;background:var(--stroke);margin:.25rem 0"></div>
        <button class="export-item" data-format="markdown">🔤 Markdown</button>
      </div>
    </div>
  </header>

  <div id="loadingSkeleton" class="skeleton">
    <div class="skeleton-line"></div><div class="skeleton-line"></div>
    <div class="skeleton-line"></div><div class="skeleton-line"></div>
    <div class="skeleton-line"></div><div class="skeleton-line"></div>
    <div class="skeleton-line"></div><div class="skeleton-line"></div>
    <div class="skeleton-line"></div>
  </div>

  <div id="content" style="display:none"></div>

  <div id="overlay">
    <div id="overlay-toolbar">
      <span style="font-weight:600;font-size:.9rem">Diagram</span>
      <div class="sep"></div>
      <span style="font-size:.82rem;color:var(--fg-muted)">Zoom:</span>
      <button id="ovZoomOut" class="btn" title="Zoom out">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path d="M5 12h14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
      </button>
      <button id="ovZoomReset" class="btn btn-text" style="font-size:.78rem">100%</button>
      <button id="ovZoomIn" class="btn" title="Zoom in">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path d="M12 5v14M5 12h14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
      </button>
      <div class="sep"></div>
      <button id="ovFitWidth"  class="btn btn-text" style="font-size:.78rem">Fit width</button>
      <button id="ovFitHeight" class="btn btn-text" style="font-size:.78rem">Fit height</button>
      <div class="grow"></div>
      <button id="ovExport" class="btn btn-text" style="font-size:.78rem">Export SVG</button>
      <button id="ovClose" class="btn" title="Close (Esc)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none"><path d="M18 6 6 18M6 6l12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
      </button>
    </div>
    <div id="overlay-panzoom"><div id="overlay-panzoom-inner"></div></div>
  </div>

  <link rel="stylesheet" href="/lib/katex.min.css">
  <script src="/lib/highlight.min.js"></script>
  <script src="/lib/katex.min.js"></script>
  <script src="/lib/auto-render.min.js"></script>
  <!-- Mermaid: UMD版は window.mermaid としてロードされる（チャンク分割なし） -->
  <script src="/lib/mermaid.min.js"></script>
  <script type="module">
    const mermaid = window.mermaid;

    // ── DOM refs ──────────────────────────────────────────────────
    const $ = id => document.getElementById(id);
    const contentEl   = $('content');
    const titleEl     = $('title');
    const statusDot   = $('statusDot');
    const themeBtn    = $('themeBtn');
    const skeleton    = $('loadingSkeleton');
    const overlay     = $('overlay');
    const ovPanzoom   = $('overlay-panzoom');
    const ovInner     = $('overlay-panzoom-inner');

    const MOON_SVG = '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>';
    const SUN_SVG  = '<circle cx="12" cy="12" r="5" stroke="currentColor" stroke-width="1.5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>';
    const COPY_ICON  = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke="currentColor" stroke-width="2"/></svg>';
    const CHECK_ICON = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path d="M20 6 9 17l-5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';
    const EXPAND_SVG = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';

    // ── State ─────────────────────────────────────────────────────
    let currentTheme = 'dark';
    let mermaidCounter = 0;
    const lastGoodSvg = {};
    let ovSvgSource = '';
    let ovBaseW = 0, ovBaseH = 0, ovZoom = 1, ovFitMode = 'width';
    let ovDragging = false, ovStartX = 0, ovStartY = 0, ovStartL = 0, ovStartT = 0;

    // ── Theme ─────────────────────────────────────────────────────
    function applyTheme(theme) {
      currentTheme = theme;
      document.documentElement.setAttribute('data-theme', theme);
      $('themeIcon').innerHTML = theme === 'dark' ? MOON_SVG : SUN_SVG;
      const hljsLink = $('hljs-theme');
      if (hljsLink) hljsLink.href = theme === 'dark'
        ? '/lib/hljs-dark.min.css'
        : '/lib/hljs-light.min.css';
      mermaid.initialize({ startOnLoad: false, theme: theme === 'dark' ? 'dark' : 'default', securityLevel: 'strict' });
    }

    themeBtn.addEventListener('click', () => {
      applyTheme(currentTheme === 'dark' ? 'light' : 'dark');
      contentEl.querySelectorAll('.mermaid-block.mermaid-rendered').forEach(b => {
        b.classList.remove('mermaid-rendered');
        renderMermaidBlock(b);
      });
    });

    // ── Mermaid ───────────────────────────────────────────────────
    async function renderMermaidBlock(block) {
      const source = decodeURIComponent(block.dataset.mermaidSource || '');
      if (!source.trim()) return;
      const wrap = block.querySelector('.mermaid-svg-wrap');
      if (!wrap) return;
      try {
        mermaidCounter++;
        const { svg } = await mermaid.render('mmd-' + mermaidCounter, source);
        wrap.innerHTML = svg;
        block.classList.add('mermaid-rendered');
        block.classList.remove('mermaid-errored');
        lastGoodSvg[block.id] = svg;
        block.querySelectorAll('.error,.error-icon,.error-text,.errorText').forEach(n => n.remove());
      } catch(e) {
        block.classList.add('mermaid-errored');
        if (lastGoodSvg[block.id]) wrap.innerHTML = lastGoodSvg[block.id];
        let errEl = block.querySelector('.mermaid-error');
        if (!errEl) { errEl = document.createElement('div'); errEl.className = 'mermaid-error'; block.appendChild(errEl); }
        errEl.textContent = String(e?.message || e || 'Invalid diagram')
          .split('\n').find(Boolean)?.replace(/\s*mermaid version[\d.\s]+$/i,'').trim() || 'Invalid diagram';
      }
    }

    async function renderAllMermaid() {
      for (const b of contentEl.querySelectorAll('.mermaid-block[data-mermaid-source]:not(.mermaid-rendered)')) {
        await renderMermaidBlock(b);
      }
    }

    // ── Code: syntax highlight + lang badge + copy button ────────
    function attachCodeFeatures(root) {
      root.querySelectorAll('.code-wrap').forEach(wrap => {
        const pre  = wrap.querySelector('pre');
        const code = pre?.querySelector('code');
        if (!pre || !code) return;

        // highlight.js でシンタックスハイライト（未処理のブロックのみ）
        if (window.hljs && !code.dataset.highlighted) {
          window.hljs.highlightElement(code);
        }

        // コピーボタン（既に付いていたらスキップ）
        if (wrap.querySelector('.copy-btn')) return;
        const btn = document.createElement('button');
        btn.className = 'copy-btn'; btn.title = 'Copy'; btn.innerHTML = COPY_ICON;
        btn.addEventListener('click', async e => {
          e.stopPropagation();
          // highlight.js 適用後もテキストは textContent で取れる
          const text = code.textContent || '';
          try {
            await navigator.clipboard.writeText(text);
            btn.innerHTML = CHECK_ICON; btn.classList.add('copied');
            setTimeout(() => { btn.innerHTML = COPY_ICON; btn.classList.remove('copied'); }, 1500);
          } catch(_) {}
        });
        wrap.appendChild(btn);
      });
    }

    // ── Content update ────────────────────────────────────────────
    // 文字列の簡易ハッシュ（ブロックの同一性判定に使う）
    function hashStr(str) {
      let h = 5381;
      for (let i = 0; i < str.length; i++) h = ((h << 5) + h + str.charCodeAt(i)) | 0;
      return (h >>> 0).toString(36) + ':' + str.length;
    }

    // 新しいブロック列を contentEl に差分適用する。
    // data-key が同じ既存ノードはそのまま使い回す（Mermaid の描画結果やスクロール位置を保つ）
    function reconcile(parent, newChildren) {
      const pool = new Map();
      for (const n of Array.from(parent.children)) {
        const k = n.dataset.key;
        if (!pool.has(k)) pool.set(k, []);
        pool.get(k).push(n);
      }
      const result = newChildren.map(n => {
        const list = pool.get(n.dataset.key);
        return list && list.length ? list.shift() : n;
      });
      result.forEach((node, i) => {
        const cur = parent.children[i];
        if (cur !== node) parent.insertBefore(node, cur || null);
      });
      while (parent.children.length > result.length) parent.lastElementChild.remove();
    }

    async function renderContent(html) {
      // 新しい内容は切り離した要素の上で全部加工してから、最後に差分だけ DOM に入れる
      const tmp = document.createElement('div');
      tmp.innerHTML = html;

      // pulldown-cmark の ```mermaid → mermaid-block に変換
      tmp.querySelectorAll('code.language-mermaid').forEach(code => {
        const source = encodeURIComponent(code.textContent || '');
        const id = 'mmd-' + hashStr(source);
        const block = document.createElement('div');
        block.className = 'mermaid-block'; block.id = id;
        block.dataset.mermaidSource = source;
        block.innerHTML =
          `<button class="mermaid-expand-btn" data-expand="${id}" title="全画面表示">${EXPAND_SVG}</button>` +
          `<div class="mermaid-svg-wrap"><p style="color:var(--fg-muted);font-size:.85rem">Rendering…</p></div>`;
        code.closest('pre').replaceWith(block);
      });

      // コードブロックに code-wrap + lang badge を付ける
      // pulldown-cmark は <pre><code class="language-rust"> の形式で出力する
      tmp.querySelectorAll('pre:not(.mermaid-block pre)').forEach(pre => {
        if (pre.closest('.code-wrap')) return;
        const code = pre.querySelector('code');
        // "language-rust" → "rust" を抽出
        const langClass = code?.className?.match(/(?:^|\s)language-(\S+)/)?.[1] || '';
        const wrap = document.createElement('div');
        wrap.className = 'code-wrap';
        pre.replaceWith(wrap);
        if (langClass) {
          const badge = document.createElement('span');
          badge.className = 'code-lang'; badge.textContent = langClass;
          wrap.appendChild(badge);
        }
        wrap.appendChild(pre);
      });

      // 加工はすべて tmp 上で行う（DOM に入れる前なのでちらつかない）
      rewriteRelativeImages(tmp);
      attachCodeFeatures(tmp);
      renderGithubAlerts(tmp);
      addHeadingAnchors(tmp);
      replaceEmoji(tmp);
      renderMath(tmp);

      // ブロックごとのキー。Mermaid はソースで、それ以外は加工後の HTML で判定する
      for (const el of Array.from(tmp.children)) {
        el.dataset.key = el.classList.contains('mermaid-block')
          ? 'mmd:' + el.dataset.mermaidSource
          : hashStr(el.outerHTML);
      }

      reconcile(contentEl, Array.from(tmp.children));
      skeleton.style.display = 'none';
      contentEl.style.display = '';

      await renderAllMermaid();
      if (pendingScroll) { scrollToLine(pendingScroll.line, pendingScroll.total); pendingScroll = null; }
    }

    // ── Overlay ───────────────────────────────────────────────────
    function openOverlay(svgHtml) {
      ovSvgSource = svgHtml;
      ovInner.innerHTML = svgHtml;
      overlay.classList.add('active');
      const svg = ovInner.querySelector('svg');
      if (svg) {
        const vb = svg.getAttribute('viewBox');
        if (vb) {
          const p = vb.trim().split(/\s+/).map(Number);
          if (p.length === 4) { ovBaseW = p[2]; ovBaseH = p[3]; }
        } else {
          ovBaseW = parseFloat(svg.getAttribute('width')) || 800;
          ovBaseH = parseFloat(svg.getAttribute('height')) || 600;
        }
      }
      ovZoom = 1; ovFitMode = 'width';
      updateOverlayFit(); updateOverlayFitBtns();
    }

    function closeOverlay() { overlay.classList.remove('active'); ovInner.innerHTML = ''; }

    function applyOverlayZoom() {
      const svg = ovInner.querySelector('svg');
      if (!svg || !ovBaseW || !ovBaseH) return;
      svg.setAttribute('width',  String(Math.max(1, Math.round(ovBaseW * ovZoom))));
      svg.setAttribute('height', String(Math.max(1, Math.round(ovBaseH * ovZoom))));
      $('ovZoomReset').textContent = Math.round(ovZoom * 100) + '%';
    }

    function updateOverlayFit() {
      if (!ovBaseW || !ovBaseH) return;
      const cw = ovPanzoom.clientWidth || ovBaseW;
      const ch = ovPanzoom.clientHeight || ovBaseH;
      if (ovFitMode === 'width')  ovZoom = cw / ovBaseW;
      if (ovFitMode === 'height') ovZoom = ch / ovBaseH;
      ovZoom = Math.min(8, Math.max(0.1, ovZoom));
      applyOverlayZoom();
    }

    function updateOverlayFitBtns() {
      $('ovFitWidth').style.borderColor  = ovFitMode === 'width'  ? 'var(--accent)' : '';
      $('ovFitHeight').style.borderColor = ovFitMode === 'height' ? 'var(--accent)' : '';
    }

    $('ovZoomIn').addEventListener('click',    () => { ovFitMode='none'; ovZoom*=1.2; applyOverlayZoom(); updateOverlayFitBtns(); });
    $('ovZoomOut').addEventListener('click',   () => { ovFitMode='none'; ovZoom/=1.2; applyOverlayZoom(); updateOverlayFitBtns(); });
    $('ovZoomReset').addEventListener('click', () => { ovFitMode='none'; ovZoom=1;    applyOverlayZoom(); updateOverlayFitBtns(); });
    $('ovFitWidth').addEventListener('click',  () => { ovFitMode = ovFitMode==='width'  ? 'none' : 'width';  updateOverlayFit(); updateOverlayFitBtns(); });
    $('ovFitHeight').addEventListener('click', () => { ovFitMode = ovFitMode==='height' ? 'none' : 'height'; updateOverlayFit(); updateOverlayFitBtns(); });
    $('ovClose').addEventListener('click', closeOverlay);
    document.addEventListener('keydown', e => { if (e.key === 'Escape') closeOverlay(); });

    // pan (drag)
    ovPanzoom.addEventListener('mousedown', e => {
      ovDragging=true; ovPanzoom.classList.add('dragging');
      ovStartX=e.clientX; ovStartY=e.clientY; ovStartL=ovPanzoom.scrollLeft; ovStartT=ovPanzoom.scrollTop;
    });
    window.addEventListener('mousemove', e => {
      if (!ovDragging) return;
      ovPanzoom.scrollLeft = ovStartL-(e.clientX-ovStartX);
      ovPanzoom.scrollTop  = ovStartT-(e.clientY-ovStartY);
    });
    window.addEventListener('mouseup', () => { ovDragging=false; ovPanzoom.classList.remove('dragging'); });

    // wheel zoom in overlay
    ovPanzoom.addEventListener('wheel', e => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault(); ovFitMode='none';
        ovZoom *= e.deltaY > 0 ? 1/1.1 : 1.1;
        applyOverlayZoom(); updateOverlayFitBtns();
      }
    }, { passive: false });

    // export SVG
    $('ovExport').addEventListener('click', () => {
      if (!ovSvgSource) return;
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([ovSvgSource], { type: 'image/svg+xml' }));
      a.download = 'diagram.svg'; a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 2000);
    });

    // ── Event delegation: expand button ──────────────────────────
    contentEl.addEventListener('click', e => {
      const btn = e.target.closest('[data-expand]');
      if (!btn) return;
      const block = document.getElementById(btn.dataset.expand);
      if (!block) return;
      const wrap = block.querySelector('.mermaid-svg-wrap');
      if (!wrap) return;
      // SVG HTML 文字列を openOverlay に渡す（mdkite 方式）
      const svgHtml = wrap.innerHTML;
      if (svgHtml && svgHtml.includes('<svg')) openOverlay(svgHtml);
    });

    // ── Front matter パネル ───────────────────────────────────────
    function updateFrontMatter(fm) {
      let panel = document.getElementById('fm-panel');
      if (!fm) {
        if (panel) panel.style.display = 'none';
        return;
      }
      if (!panel) {
        panel = document.createElement('details');
        panel.id = 'fm-panel';
        panel.innerHTML =
          '<summary>Front matter</summary>' +
          '<pre id="fm-content"></pre>';
        contentEl.parentNode.insertBefore(panel, contentEl);
      }
      const pre = document.getElementById('fm-content');
      if (pre) pre.textContent = fm;
      panel.style.display = '';
    }

    // ── GitHub Alerts ─────────────────────────────────────────────
    const ALERT_ICONS = {
      note:      '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>',
      tip:       '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.595h4.017c.084-.699.337-1.184.621-1.595.203-.292.45-.584.673-.848.29-.342.214-.253.214-.253.56-.679.984-1.32.984-2.304 0-2.06-1.637-3.75-4-3.75ZM6 7.058C5.763 6.66 5.5 6.04 5.5 5.25c0-1.311.989-2.25 2.5-2.25s2.5.939 2.5 2.25c0 .79-.263 1.41-.5 1.808V7.5h-4V7.058Zm2 7.942a1.5 1.5 0 0 1-1.422-1h2.844A1.5 1.5 0 0 1 8 15Zm3.479-4H4.521A5.578 5.578 0 0 0 4.5 11h7a5.578 5.578 0 0 0-.021.058Z"/></svg>',
      warning:   '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>',
      caution:   '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>',
      important: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>',
    };
    const ALERT_LABEL = { note:'Note', tip:'Tip', warning:'Warning', caution:'Caution', important:'Important' };

    function renderGithubAlerts(root) {
      root.querySelectorAll('blockquote').forEach(bq => {
        const firstP = bq.querySelector('p');
        if (!firstP) return;
        // pulldown-cmark: "[!NOTE]\n..." or "[!NOTE]<br>..."
        const match = firstP.innerHTML.match(/^\[!(NOTE|TIP|WARNING|CAUTION|IMPORTANT)\](<br\s*\/?>|\n)?/i);
        if (!match) return;
        const type = match[1].toLowerCase();
        bq.classList.add('markdown-alert', `markdown-alert-${type}`);
        // [!TYPE] テキストを除去
        firstP.innerHTML = firstP.innerHTML.replace(/^\[!(NOTE|TIP|WARNING|CAUTION|IMPORTANT)\](<br\s*\/?>|\n)?/i, '');
        if (!firstP.innerHTML.trim()) firstP.remove();
        // タイトルを先頭に挿入
        const title = document.createElement('div');
        title.className = 'markdown-alert-title';
        title.innerHTML = (ALERT_ICONS[type] || '') + ' ' + ALERT_LABEL[type];
        bq.insertBefore(title, bq.firstChild);
      });
    }

    // ── 見出しアンカーリンク ──────────────────────────────────────
    function addHeadingAnchors(root) {
      root.querySelectorAll('h1,h2,h3,h4,h5,h6').forEach(h => {
        // 既にアンカーがあればスキップ
        if (h.querySelector('.anchor-link')) return;
        const id = h.textContent.trim()
          .toLowerCase()
          .replace(/\s+/g, '-')
          .replace(/[^\w\u3040-\u30FF\u3400-\u4DBF\u4E00-\u9FFF-]/g, '');
        if (!id) return;
        h.id = id;
        const a = document.createElement('a');
        a.className = 'anchor-link';
        a.href = '#' + id;
        a.title = 'この見出しへのリンク';
        a.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="m7.775 3.275 1.25-1.25a3.5 3.5 0 1 1 4.95 4.95l-2.5 2.5a3.5 3.5 0 0 1-4.95 0 .751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018 1.998 1.998 0 0 0 2.83 0l2.5-2.5a2.002 2.002 0 0 0-2.83-2.83l-1.25 1.25a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042Zm-4.69 9.64a1.998 1.998 0 0 0 2.83 0l1.25-1.25a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042l-1.25 1.25a3.5 3.5 0 1 1-4.95-4.95l2.5-2.5a3.5 3.5 0 0 1 4.95 0 .751.751 0 0 1-.018 1.042.751.751 0 0 1-1.042.018 1.998 1.998 0 0 0-2.83 0l-2.5 2.5a1.998 1.998 0 0 0 0 2.83Z"/></svg>';
        h.appendChild(a);
      });
    }

    // ── 絵文字 :name: → Unicode ──────────────────────────────────
    const EMOJI = {
      '+1':'👍','-1':'👎','100':'💯','tada':'🎉','sparkles':'✨','star':'⭐',
      'heart':'❤️','fire':'🔥','rocket':'🚀','check':'✅','x':'❌','warning':'⚠️',
      'bulb':'💡','eyes':'👀','wave':'👋','clap':'👏','pray':'🙏','muscle':'💪',
      'smile':'😊','grin':'😁','laughing':'😆','wink':'😉','blush':'😊',
      'sweat_smile':'😅','joy':'😂','rofl':'🤣','sob':'😭','cry':'😢',
      'angry':'😠','rage':'😡','confused':'😕','thinking':'🤔','nerd_face':'🤓',
      'sunglasses':'😎','partying_face':'🥳','raised_hands':'🙌','ok_hand':'👌',
      'point_right':'👉','point_left':'👈','point_up':'👆','point_down':'👇',
      'thumbsup':'👍','thumbsdown':'👎','v':'✌️','crossed_fingers':'🤞',
      'bug':'🐛','memo':'📝','pencil':'✏️','book':'📖','books':'📚',
      'computer':'💻','keyboard':'⌨️','phone':'📱','email':'📧','link':'🔗',
      'lock':'🔒','unlock':'🔓','key':'🔑','hammer':'🔨','wrench':'🔧',
      'gear':'⚙️','package':'📦','inbox_tray':'📥','outbox_tray':'📤',
      'white_check_mark':'✅','ballot_box_with_check':'☑️','negative_squared_cross_mark':'❎',
      'information_source':'ℹ️','question':'❓','exclamation':'❗',
      'arrow_up':'⬆️','arrow_down':'⬇️','arrow_left':'⬅️','arrow_right':'➡️',
      'zap':'⚡','sunny':'☀️','cloud':'☁️','snowflake':'❄️','umbrella':'☂️',
      'coffee':'☕','pizza':'🍕','beer':'🍺','cake':'🎂','tada':'🎉',
      'trophy':'🏆','medal_sports':'🏅','dart':'🎯','game_die':'🎲',
      'art':'🎨','musical_note':'🎵','microphone':'🎤','headphones':'🎧',
      'earth_asia':'🌏','earth_americas':'🌎','earth_africa':'🌍','globe_with_meridians':'🌐',
      'japan':'🗾','construction':'🚧','no_entry':'⛔','stop_sign':'🛑',
      'recycle':'♻️','sos':'🆘','new':'🆕','up':'🆙','cool':'🆒','free':'🆓',
      'abc':'🔤','1234':'🔢','symbols':'🔣','capital_abcd':'🔠',
      'white_square_button':'🔳','black_square_button':'🔲',
    };

    function replaceEmoji(root) {
      // テキストノードのみ走査（code/pre 内は除外）
      const walker = document.createTreeWalker(
        root, NodeFilter.SHOW_TEXT,
        { acceptNode: n => {
          const p = n.parentElement;
          if (!p) return NodeFilter.FILTER_REJECT;
          if (p.closest('code,pre,.katex')) return NodeFilter.FILTER_REJECT;
          return NodeFilter.FILTER_ACCEPT;
        }}
      );
      const nodes = [];
      while (walker.nextNode()) nodes.push(walker.currentNode);
      nodes.forEach(node => {
        if (!node.textContent.includes(':')) return;
        const replaced = node.textContent.replace(/:([a-z0-9_+\-]+):/g, (m, name) => EMOJI[name] || m);
        if (replaced !== node.textContent) node.textContent = replaced;
      });
    }

    // ── KaTeX 数式レンダリング ────────────────────────────────────
    function renderMath(root) {
      if (!window.renderMathInElement) return;
      renderMathInElement(root, {
        delimiters: [
          { left: '$$', right: '$$', display: true },
          { left: '$',  right: '$',  display: false },
          { left: '\\[', right: '\\]', display: true },
          { left: '\\(', right: '\\)', display: false },
        ],
        throwOnError: false,
        // コードブロック内はスキップ
        ignoredTags: ['script','noscript','style','textarea','pre','code'],
      });
    }

    // ── 相対パス画像を /assets?p= に書き換え ────────────────────
    function rewriteRelativeImages(root) {
      root.querySelectorAll('img[src]').forEach(img => {
        const src = img.getAttribute('src');
        // 絶対URLや data URI はスキップ
        if (!src || /^(?:[a-zA-Z][a-zA-Z0-9+.-]*:|\/\/|\/|#|data:)/.test(src)) return;
        img.setAttribute('src', `/assets?p=${encodeURIComponent(src)}`);
      });
    }

    // ── スクロール同期（手動スクロール中は一時停止） ─────────────
    let scrollSyncPaused = false;
    let scrollSyncTimer  = null;
    window.addEventListener('wheel',     () => { scrollSyncPaused = true; clearTimeout(scrollSyncTimer); scrollSyncTimer = setTimeout(() => { scrollSyncPaused = false; }, 3000); }, { passive: true });
    window.addEventListener('touchmove', () => { scrollSyncPaused = true; clearTimeout(scrollSyncTimer); scrollSyncTimer = setTimeout(() => { scrollSyncPaused = false; }, 3000); }, { passive: true });

    // 内容更新の直後に届いたスクロールは、描画が終わってから反映する
    let pendingScroll = null;
    let lastScroll = { line: 1, total: 1 };

    // 行マーカー（<span class="src-line" data-line>）の一覧を行番号順に返す
    function lineMarkers() {
      return Array.from(contentEl.querySelectorAll('.src-line'))
        .map(el => ({ line: +el.dataset.line, top: el.getBoundingClientRect().top + window.scrollY }))
        .filter(m => Number.isFinite(m.line))
        .sort((a, b) => a.line - b.line);
    }

    // カーソル行 line に対応するページ上の Y 座標。
    // 行番号がぴったり一致するマーカーが無ければ、前後のマーカーの間を行数で按分する
    function yForLine(line, total) {
      const markers = lineMarkers();
      if (!markers.length) {
        const ratio = Math.max(0, Math.min(1, (line - 1) / Math.max(1, total - 1)));
        return (document.documentElement.scrollHeight - window.innerHeight) * ratio;
      }
      let before = null, after = null;
      for (const m of markers) {
        if (m.line <= line) before = m; else { after = m; break; }
      }
      if (!before) return 0;
      const endTop = contentEl.getBoundingClientRect().bottom + window.scrollY;
      const nextLine = after ? after.line : total + 1;
      const nextTop  = after ? after.top  : endTop;
      const span = Math.max(1, nextLine - before.line);
      return before.top + (nextTop - before.top) * Math.min(1, (line - before.line) / span);
    }

    function scrollToLine(line, total) {
      lastScroll = { line, total };
      if (scrollSyncPaused) return;
      // カーソル行が画面の上から 30% の位置に来るようにする
      const y = yForLine(line, total) - window.innerHeight * 0.3;
      window.scrollTo({ top: Math.max(0, y), behavior: 'instant' });
    }

    // ── クリックでエディタの該当行へジャンプ ──────────────────────
    contentEl.classList.add('click-sync');
    contentEl.addEventListener('click', e => {
      // リンク・ボタン・図の操作、テキスト選択中はジャンプしない
      if (e.target.closest('a,button,input,details,summary,.mermaid-block,.copy-btn')) return;
      if (String(window.getSelection?.() || '').length) return;
      const y = e.clientY + window.scrollY;
      let best = null;
      for (const el of contentEl.querySelectorAll('.src-line')) {
        const top = el.getBoundingClientRect().top + window.scrollY;
        if (top <= y + 1 && (!best || top >= best.top)) best = { top, line: +el.dataset.line };
      }
      if (best && wsConn && wsConn.readyState === WebSocket.OPEN) {
        wsConn.send(JSON.stringify({ type: 'goto', line: best.line }));
      }
    });

    // ── WebSocket ─────────────────────────────────────────────────
    let wsConn = null;
    let rendering = false;
    function connect() {
      const ws = new WebSocket(`ws://${location.host}/ws`);
      wsConn = ws;
      ws.onopen  = () => { statusDot.className = 'status-dot connected'; statusDot.title = 'Connected'; };
      ws.onmessage = async e => {
        const data = JSON.parse(e.data);

        // スクロール同期メッセージ
        if (data.type === 'scroll') {
          if (rendering) pendingScroll = { line: data.line, total: data.total };
          else scrollToLine(data.line, data.total);
          return;
        }

        // コンテンツ更新
        document.title = data.title;
        titleEl.textContent = data.title;
        updateFrontMatter(data.front_matter || null);
        rendering = true;
        try { await renderContent(data.html); } finally { rendering = false; }
      };
      ws.onclose = () => { statusDot.className = 'status-dot error'; statusDot.title = 'Disconnected'; setTimeout(connect, 3000); };
      ws.onerror = () => ws.close();
    }

    applyTheme('dark');
    connect();

    // ── Export ────────────────────────────────────────────────────
    const exportBtn  = document.getElementById('exportBtn');
    const exportMenu = document.getElementById('exportMenu');

    exportBtn.addEventListener('click', e => {
      e.stopPropagation();
      exportMenu.style.display = exportMenu.style.display === 'none' ? 'block' : 'none';
    });
    document.addEventListener('click', () => { exportMenu.style.display = 'none'; });

    // type="module" のスクリプトは onclick 属性から関数を参照できないので、リスナーで登録する
    exportMenu.querySelectorAll('[data-format]').forEach(item => {
      item.addEventListener('click', () => exportAs(item.dataset.format));
    });

    // fetch で取得してからダウンロードする（失敗時にプレビュー画面がエラー JSON に遷移しないように）
    async function exportAs(format) {
      exportMenu.style.display = 'none';
      const label = exportBtn.lastChild.textContent;
      exportBtn.disabled = true;
      exportBtn.lastChild.textContent = ' 変換中…';
      try {
        const res = await fetch(`/export?format=${encodeURIComponent(format)}`);
        if (!res.ok) {
          let msg = `HTTP ${res.status}`;
          try { msg = (await res.json()).error || msg; } catch (_) {}
          throw new Error(msg);
        }
        const disposition = res.headers.get('Content-Disposition') || '';
        // 日本語名を保つため filename*（RFC 5987）を優先し、無ければ filename を使う
        const star  = disposition.match(/filename\*=UTF-8''([^;]+)/);
        const plain = disposition.match(/filename="([^"]+)"/);
        const filename = star ? decodeURIComponent(star[1]) : plain ? plain[1] : `export.${format}`;
        const url = URL.createObjectURL(await res.blob());
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        link.remove();
        setTimeout(() => URL.revokeObjectURL(url), 10000);
      } catch (e) {
        alert(`エクスポートに失敗しました (${format})\n\n${e.message}`);
      } finally {
        exportBtn.disabled = false;
        exportBtn.lastChild.textContent = label;
      }
    }
  </script>
</body>
</html>
"#;

