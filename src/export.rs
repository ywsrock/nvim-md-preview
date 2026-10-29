//! エクスポート（Markdown / pandoc / Typst / ブラウザでの PDF 印刷）

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::{path::{Path, PathBuf}, sync::Arc};
use crate::state::{AppState, ExportOptions};

/// /export?format=pdf|docx|html|epub|markdown — エクスポートハンドラ
#[derive(Deserialize)]
pub(crate) struct ExportParams {
    format: String,
}

pub(crate) async fn export_handler(
    Query(params): Query<ExportParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    match export_bytes(&state, &params.format.to_lowercase()).await {
        Ok((data, filename)) => download_response(data, &filename, mime_for_format(&filename)),
        Err(e) => error_response(&e),
    }
}

/// 形式名 → 拡張子（"md" / "markdown" はどちらも .md）
pub(crate) fn extension_for(format: &str) -> Option<&'static str> {
    match format {
        "markdown" | "md" => Some("md"),
        "docx" => Some("docx"),
        "html" => Some("html"),
        "epub" => Some("epub"),
        "pdf"  => Some("pdf"),
        _ => None,
    }
}

pub(crate) fn mime_for_format(filename: &str) -> &'static str {
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
        return Err(format!("{}: {}", "Unsupported format", format));
    };

    if *state.view_kind.read().await != "markdown" {
        return Err("Export is available for Markdown files only.".to_string());
    }

    // プレビュー中の内容（未保存分も含む）を使う
    let content = state.current_content.read().await.clone()
        .ok_or_else(|| "No file is open. Start the preview with :MdPreview.".to_string())?;
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
    tokio::fs::create_dir_all(&work).await
        .map_err(|e| format!("{}: {}", "Failed to create a temporary directory", e))?;
    let md_tmp = work.join("input.md");
    tokio::fs::write(&md_tmp, &content).await
        .map_err(|e| format!("{}: {}", "Failed to create a temporary file", e))?;

    let browser = state.browser_path.read().await.clone();
    let pdf_engine = state.export_opts.read().await.pdf_engine.clone();
    let print_browser = browser.filter(|_| ext == "pdf" && pdf_engine == "browser");
    let result = if let Some(browser) = print_browser {
        let token = state.access.token.clone();
        let port = state.port.load(std::sync::atomic::Ordering::Relaxed);
        print_with_browser(&browser, port, token.as_deref(), &work).await
    } else if ext == "pdf" {
        // パスはコピーしてすぐロックを外す（変換中に設定の更新を止めないように）
        let typst = state.typst_path.read().await.clone();
        let typst_bin = typst.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "typst".to_string());
        let opts = state.export_opts.read().await.clone();
        let packages = state.typst_packages.read().await.clone();
        convert_with_typst(&typst_bin, &work, base_dir.as_deref(), &opts, packages.as_deref()).await
    } else {
        let pandoc = state.pandoc_path.read().await.clone();
        let pandoc_bin = pandoc.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "pandoc".to_string());
        let opts = state.export_opts.read().await.clone();
        convert_with_pandoc(&pandoc_bin, &work, ext, base_dir.as_ref(), &opts).await
    };

    let _ = tokio::fs::remove_dir_all(&work).await;
    result.map(|data| (data, filename))
}

pub(crate) fn unique_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// pandoc で work/input.md を変換して出力バイト列を返す
pub(crate) async fn convert_with_pandoc(
    pandoc_bin: &str,
    work: &Path,
    format: &str,
    base_dir: Option<&PathBuf>,
    opts: &ExportOptions,
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
    if opts.toc {
        args.push("--toc".to_string());
    }
    if opts.numbering {
        args.push("--number-sections".to_string());
    }

    let output = tokio::process::Command::new(pandoc_bin)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("{}\n\n{}: {}",
            "pandoc was not found. Install it with :MdPreviewInstall.",
            "Error", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{}\n\n{}: {}",
            "pandoc failed to convert the document. Check that pandoc is installed (:MdPreviewInstall).",
            "Error",
            truncate_chars(&stderr, 500)
        ));
    }
    tokio::fs::read(&out_path).await
        .map_err(|_| "Failed to read the converted file.".to_string())
}

/// Chromium 系ブラウザを画面なしで起動し、/print を PDF に書き出す
/// ふだん使っているブラウザに影響しないよう、一時ディレクトリのプロフィールで起動する
pub(crate) async fn print_with_browser(browser: &Path, port: u16, token: Option<&str>, work: &Path) -> Result<Vec<u8>, String> {
    let pdf_path = work.join("output.pdf");
    let url = match token {
        Some(t) => format!("http://127.0.0.1:{}/print?token={}", port, t),
        None => format!("http://127.0.0.1:{}/print", port),
    };
    let run = |no_sandbox: bool| {
        let mut cmd = tokio::process::Command::new(browser);
        cmd.arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-extensions")
            .arg(format!("--user-data-dir={}", work.join("browser-profile").display()))
            .arg("--no-pdf-header-footer")
            .arg("--print-to-pdf-no-header")
            // Mermaid や数式の描画が終わるまで待ってから印刷する
            .arg("--virtual-time-budget=20000")
            .arg(format!("--print-to-pdf={}", pdf_path.display()));
        if no_sandbox {
            cmd.arg("--no-sandbox");
        }
        cmd.arg(&url).kill_on_drop(true);
        cmd
    };

    let attempt = |no_sandbox: bool| {
        let mut cmd = run(no_sandbox);
        async move {
            match tokio::time::timeout(std::time::Duration::from_secs(90), cmd.output()).await {
                Err(_) => Err("The browser did not finish printing within 90 seconds.".to_string()),
                Ok(Err(e)) => Err(format!("Failed to start the browser: {}", e)),
                Ok(Ok(out)) => Ok(out),
            }
        }
    };

    let mut output = attempt(false).await?;
    // Linux では、ユーザー名前空間が使えない環境（コンテナや一部の CI）でサンドボックスの起動に失敗することがある
    if cfg!(target_os = "linux") && !pdf_path.exists() {
        output = attempt(true).await?;
    }
    match tokio::fs::read(&pdf_path).await {
        Ok(data) if !data.is_empty() => Ok(data),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "The browser could not print the PDF. Set export.pdf_engine = \"typst\" to use Typst instead.\n\nError: {}",
                truncate_chars(&stderr, 500)
            ))
        }
    }
}

/// Typst の文字列リテラルに埋め込めるようにエスケープする
pub(crate) fn typst_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// パスを (Typst の --root, root からの相対パス) に分ける。
/// Typst は root の外のファイルを読めないので、相対パス画像のために root をドライブ / ファイルシステムの根にする
pub(crate) fn split_root(path: &Path) -> Option<(PathBuf, String)> {
    let mut comps = path.components();
    let mut root = PathBuf::new();
    for c in comps.by_ref() {
        root.push(c.as_os_str());
        if matches!(c, std::path::Component::RootDir) { break; }
    }
    let rel: Vec<String> = comps.map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    Some((root, rel.join("/")))
}

/// PDF 用の Typst ソースを組み立てる
pub(crate) fn build_typst_source(opts: &ExportOptions, image_prefix: Option<&str>) -> String {
    let font = if opts.font.is_empty() {
        r#"("Noto Serif CJK JP", "Noto Serif", "Georgia")"#.to_string()
    } else {
        format!("({}, \"Noto Serif CJK JP\")", typst_str(&opts.font))
    };
    let numbering = if opts.numbering { r#""1.1""# } else { "none" };
    // 相対パスの画像を Markdown ファイルの場所から読む（root からの絶対パスに直す）
    let scope = match image_prefix {
        Some(prefix) => format!(
            r#", scope: (image: (path, alt: none) => image(if path.starts-with("/") or path.contains("://") {{ path }} else {{ {} + path }}, alt: alt))"#,
            typst_str(&format!("/{}/", prefix.trim_matches('/')))
        ),
        None => String::new(),
    };
    let body = format!(r#"#render(read("input.md"), h1-level: 1{})"#, scope);
    let toc = if opts.toc { "#outline()\n#pagebreak()\n" } else { "" };

    if opts.template.is_some() {
        // ユーザーテンプレート: template.typ の `template` 関数を show ルールとして適用する
        return format!(
            "#import \"@preview/cmarker:0.1.4\": render\n#import \"template.typ\": template\n#show: template\n{}{}\n",
            toc, body
        );
    }
    format!(
        r#"#import "@preview/cmarker:0.1.4": render
#set page(paper: {paper}, margin: {margin})
#set text(font: {font}, size: {size}pt)
#set heading(numbering: {numbering})

{toc}{body}
"#,
        paper = typst_str(&opts.paper),
        margin = opts.margin.parse::<f64>().map(|n| format!("{}cm", n)).unwrap_or_else(|_| opts.margin.clone()),
        font = font,
        size = opts.font_size,
        numbering = numbering,
        toc = toc,
        body = body,
    )
}

/// Typst で work/input.md を PDF に変換して出力バイト列を返す
pub(crate) async fn convert_with_typst(
    typst_bin: &str,
    work: &Path,
    base_dir: Option<&Path>,
    opts: &ExportOptions,
    packages: Option<&Path>,
) -> Result<Vec<u8>, String> {
    // Typst の cmarker パッケージで Markdown をそのまま処理
    // Markdown は raw ブロックに埋め込むと ``` を含む文書で壊れるので、ファイルを read() で読む
    let typ_path = work.join("input.typ");
    let pdf_path = work.join("output.pdf");

    if let Some(tpl) = &opts.template {
        tokio::fs::copy(tpl, work.join("template.typ")).await
            .map_err(|e| format!("{}: {}\n{}", "Cannot read the template", tpl.display(), e))?;
    }

    // 画像用の root は作業ディレクトリと同じ根（ドライブ）にある場合だけ設定する
    let roots = base_dir.and_then(split_root).zip(split_root(work));
    let (root, image_prefix) = match roots {
        Some(((base_root, base_rel), (work_root, _))) if base_root == work_root => (Some(base_root), Some(base_rel)),
        _ => (None, None),
    };

    let typ_content = build_typst_source(opts, image_prefix.as_deref());
    tokio::fs::write(&typ_path, typ_content).await
        .map_err(|e| format!("{}: {}", "Failed to create a temporary file", e))?;

    let mut args: Vec<String> = vec!["compile".into()];
    // 同梱したパッケージがあれば、そこから読む（ネットワークに取りに行かない）
    if let Some(dir) = packages.filter(|d| d.is_dir()) {
        args.push("--package-cache-path".into());
        args.push(dir.to_string_lossy().to_string());
    }
    if let Some(root) = root {
        args.push("--root".into());
        args.push(root.to_string_lossy().to_string());
    }
    args.push(typ_path.to_string_lossy().to_string());
    args.push(pdf_path.to_string_lossy().to_string());

    let output = tokio::process::Command::new(typst_bin)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("{}\n\n{}: {}",
            "Typst was not found. Install it with :MdPreviewInstall.",
            "Error", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{}\n\n{}: {}",
            "Typst failed to convert the document. Install Typst with :MdPreviewInstall.",
            "Error",
            truncate_chars(&stderr, 500)
        ));
    }
    tokio::fs::read(&pdf_path).await
        .map_err(|_| "Failed to read the PDF.".to_string())
}

/// 文字境界を壊さずに先頭 max 文字を返す（バイト単位スライスは多バイト文字で panic する）
pub(crate) fn truncate_chars(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

/// Content-Disposition を組み立てる
/// ヘッダー値は ASCII のみ許されるので、filename は ASCII に置き換え、
/// 元の名前は RFC 5987 の filename* にパーセントエンコードして入れる
pub(crate) fn content_disposition(filename: &str) -> String {
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
pub(crate) fn download_response(data: Vec<u8>, filename: &str, mime: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (header::CONTENT_DISPOSITION, content_disposition(filename)),
        ],
        data,
    ).into_response()
}

/// エラーレスポンスを JSON で返す
pub(crate) fn error_response(msg: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({"error": msg}).to_string(),
    ).into_response()
}
