mod markdown;
mod server;

use server::{AppState, NvimNotify};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{broadcast, mpsc},
};

const DEFAULT_PORT: u16 = 9876;

/// Neovim msgpack-rpc の通知メッセージを stdin から読んで処理する。
/// nvim-rs / parity-tokio-ipc に依存せず、全 OS で動作する。
///
/// msgpack-rpc notify フォーマット: [2, method, params]
///   - type=2: notification
///   - method: String
///   - params: Array
///
/// stdin は任意の位置で分割されて届き、1 回の read に複数メッセージが乗ることもあるので、
/// 読み残しをバッファに保持し、揃ったメッセージから順にデコードする。
async fn run_rpc_loop(state: Arc<AppState>) {
    // 最大 16MB まで（Neovim からの通常のメッセージはずっと小さい）
    const MAX_SIZE: usize = 16 * 1024 * 1024;

    let mut stdin = tokio::io::stdin();
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    let mut chunk = vec![0u8; 64 * 1024];

    loop {
        // バッファにある完全なメッセージをすべて処理する
        loop {
            if buf.is_empty() { break; }
            let mut cursor = std::io::Cursor::new(&buf[..]);
            match rmpv::decode::read_value(&mut cursor) {
                Ok(msg) => {
                    let consumed = cursor.position() as usize;
                    buf.drain(..consumed);
                    dispatch(&state, msg).await;
                }
                // まだデータが足りない → 続きを読む
                Err(_) => break,
            }
        }

        if buf.len() >= MAX_SIZE {
            break; // 壊れたデータでバッファが膨らみ続けるのを防ぐ
        }
        match stdin.read(&mut chunk).await {
            Ok(0) | Err(_) => break, // stdin が閉じた（Neovim 終了）
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
}

/// 1 メッセージを method ごとの処理に振り分ける
async fn dispatch(state: &Arc<AppState>, msg: rmpv::Value) {
    // [type, method, params] の形式を確認
    let arr = match msg.as_array() {
        Some(a) if a.len() >= 3 => a,
        _ => return,
    };

    // type=0: request, type=1: response, type=2: notification
    // request/response は現在無視（ping 対応は不要）
    if arr[0].as_u64().unwrap_or(0) != 2 {
        return;
    }
    let Some(method) = arr[1].as_str() else { return; };
    let Some(params) = arr[2].as_array() else { return; };

    match method {
        "md_preview_update" => handle_update(state, params).await,
        "md_preview_scroll" => handle_scroll(state, params),
        "md_preview_config" => handle_config(state, params).await,
        "md_preview_diff" => handle_diff(state, params).await,
        "md_preview_export" => {
            // 変換は時間がかかるので RPC ループを止めないよう別タスクで実行
            let state = Arc::clone(state);
            let params = params.clone();
            tokio::spawn(async move { handle_export(&state, &params).await; });
        }
        _ => {}
    }
}

/// Neovim へ msgpack-rpc 通知を stdout に書く
/// notify フォーマット: [2, method, params]。API メソッド（nvim_exec_lua など）も通知で呼べる
async fn run_nvim_writer(mut rx: mpsc::UnboundedReceiver<NvimNotify>) {
    let mut stdout = tokio::io::stdout();
    while let Some((method, params)) = rx.recv().await {
        let msg = rmpv::Value::Array(vec![
            rmpv::Value::from(2),
            rmpv::Value::from(method),
            rmpv::Value::Array(params),
        ]);
        let mut buf = Vec::new();
        if rmpv::encode::write_value(&mut buf, &msg).is_err() { continue; }
        if stdout.write_all(&buf).await.is_err() { break; }
        let _ = stdout.flush().await;
    }
}

/// `md_preview_update` 処理
async fn handle_update(state: &AppState, params: &[rmpv::Value]) {
    let Some(content) = params.first().and_then(|v| v.as_str()) else { return; };
    let title        = params.get(1).and_then(|v| v.as_str()).unwrap_or("md Preview");
    let base_dir_str = params.get(2).and_then(|v| v.as_str()).unwrap_or("");
    let file_path_str = params.get(3).and_then(|v| v.as_str()).unwrap_or("");

    if !base_dir_str.is_empty() {
        *state.base_dir.write().await = Some(PathBuf::from(base_dir_str));
    }

    // 現在のファイルパスを保存（エクスポートに使用）
    if !file_path_str.is_empty() {
        *state.current_file.write().await = Some(PathBuf::from(file_path_str));
    }

    *state.current_content.write().await = Some(content.to_string());

    let (front_matter, body) = markdown::extract_front_matter(content);
    let line_offset = markdown::front_matter_line_count(front_matter.as_deref());
    let render_opts = *state.render_opts.read().await;
    let html = markdown::to_html(&body, line_offset, render_opts);

    let payload = serde_json::json!({
        "type": "content",
        "title": title,
        "html": html,
        "front_matter": front_matter,
    }).to_string();

    state.update_html(payload).await;
}

/// `md_preview_config` 処理 — エンジンパスを受け取る
/// params: [pandoc_path, typst_path, options]
/// options（省略可）: { wiki_links, browser_keys, theme, custom_css, export = { paper, margin, font, font_size, toc, numbering, template } }
async fn handle_config(state: &AppState, params: &[rmpv::Value]) {
    let pandoc_str = params.first().and_then(|v| v.as_str()).unwrap_or("");
    let typst_str  = params.get(1).and_then(|v| v.as_str()).unwrap_or("");

    if !pandoc_str.is_empty() {
        *state.pandoc_path.write().await = Some(PathBuf::from(pandoc_str));
    }
    if !typst_str.is_empty() {
        *state.typst_path.write().await = Some(PathBuf::from(typst_str));
    }

    let Some(opts) = params.get(2) else { return; };
    let get = |v: &rmpv::Value, key: &str| -> Option<rmpv::Value> {
        v.as_map()?.iter().find(|(k, _)| k.as_str() == Some(key)).map(|(_, v)| v.clone())
    };
    let get_str = |v: &rmpv::Value, key: &str| get(v, key).and_then(|x| x.as_str().map(str::to_string));
    let get_bool = |v: &rmpv::Value, key: &str| get(v, key).and_then(|x| x.as_bool());
    let get_num = |v: &rmpv::Value, key: &str| get(v, key)
        .and_then(|x| x.as_f64().or_else(|| x.as_i64().map(|n| n as f64)).or_else(|| x.as_u64().map(|n| n as f64)));

    state.render_opts.write().await.wiki_links = get_bool(opts, "wiki_links").unwrap_or(false);
    *state.browser_keys.write().await = get_bool(opts, "browser_keys").unwrap_or(true);
    if let Some(formats) = get(opts, "export_formats").and_then(|v| v.as_array().cloned()) {
        *state.export_formats.write().await =
            formats.iter().filter_map(|f| f.as_str().map(str::to_string)).collect();
    }
    *state.theme.write().await = get_str(opts, "theme").unwrap_or_else(|| "auto".to_string());
    *state.custom_css.write().await = get_str(opts, "custom_css").unwrap_or_default();

    if let Some(ex) = get(opts, "export") {
        let mut eo = state.export_opts.write().await;
        if let Some(v) = get_str(&ex, "pdf_engine") { eo.pdf_engine = v; }
        if let Some(v) = get_str(&ex, "paper")    { eo.paper = v; }
        if let Some(v) = get_str(&ex, "margin")   { eo.margin = v; }
        if let Some(v) = get_str(&ex, "font")     { eo.font = v; }
        if let Some(v) = get_num(&ex, "font_size") { eo.font_size = v; }
        if let Some(v) = get_bool(&ex, "toc")      { eo.toc = v; }
        if let Some(v) = get_bool(&ex, "numbering") { eo.numbering = v; }
        eo.template = get_str(&ex, "template").filter(|s| !s.is_empty()).map(PathBuf::from);
        drop(eo);
        *state.browser_path.write().await = get_str(&ex, "browser_path").filter(|s| !s.is_empty()).map(PathBuf::from);
        *state.typst_packages.write().await = get_str(&ex, "typst_packages").filter(|s| !s.is_empty()).map(PathBuf::from);
    }
}

/// `md_preview_export` 処理 — バッファ内容を変換してファイルに書き出す
/// params: [format, out_path]
/// 完了したら Neovim 側の require("md_preview")._on_export_done(ok, path, err) を呼ぶ
async fn handle_export(state: &AppState, params: &[rmpv::Value]) {
    let format   = params.first().and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
    let out_path = params.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();

    let result = match server::export_bytes(state, &format).await {
        Ok((data, _filename)) => tokio::fs::write(&out_path, data).await
            .map_err(|e| format!("{}: {}", "Failed to write the file", e)),
        Err(e) => Err(e),
    };

    let (ok, err) = match result {
        Ok(()) => (true, String::new()),
        Err(e) => (false, e),
    };
    state.notify_nvim(
        "nvim_exec_lua",
        vec![
            rmpv::Value::from("require('md_preview')._on_export_done(...)"),
            rmpv::Value::Array(vec![
                rmpv::Value::from(ok),
                rmpv::Value::from(out_path),
                rmpv::Value::from(err),
            ]),
        ],
    );
}

/// `md_preview_diff` 処理 — Git の差分を受け取ってブラウザに送る
/// params: [hunks, info]
///   hunks = [[s, e, kind, old_md, ctx_s, ctx_e], ...]（kind: "add" | "mod" | "del"）
///   info  = { enabled, base, available, in_repo, colors = { add, mod, del } }
/// 変更前の Markdown（old_md）はここで HTML にしてから送る
async fn handle_diff(state: &AppState, params: &[rmpv::Value]) {
    let num = |v: Option<&rmpv::Value>| v.and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n.max(0) as u64)));
    let render_opts = *state.render_opts.read().await;
    let hunks: Vec<serde_json::Value> = params.first()
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|h| {
            let h = h.as_array()?;
            let kind = h.get(2)?.as_str()?;
            let old_md = h.get(3).and_then(|v| v.as_str()).unwrap_or("");
            let old_html = if old_md.is_empty() { String::new() } else { markdown::to_html(old_md, 0, render_opts) };
            Some(serde_json::json!({
                "s": num(h.first())?, "e": num(h.get(1))?, "kind": kind,
                "old_html": old_html,
                "ctx": [num(h.get(4)).unwrap_or(0), num(h.get(5)).unwrap_or(0)],
            }))
        }).collect())
        .unwrap_or_default();

    // info は map。値を JSON に写す（色は文字列のみ）
    let info = params.get(1);
    let get = |key: &str| info.and_then(|v| v.as_map())
        .and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)).map(|(_, v)| v.clone()));
    let color = |key: &str| get("colors")
        .and_then(|c| c.as_map().and_then(|m| m.iter().find(|(k, _)| k.as_str() == Some(key)).map(|(_, v)| v.clone())))
        .and_then(|v| v.as_str().map(str::to_string))
        .filter(|c| c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit()));

    let payload = serde_json::json!({
        "type": "diff",
        "enabled": get("enabled").and_then(|v| v.as_bool()).unwrap_or(!hunks.is_empty()),
        "available": get("available").and_then(|v| v.as_bool()).unwrap_or(true),
        "in_repo": get("in_repo").and_then(|v| v.as_bool()).unwrap_or(true),
        "base": get("base").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "HEAD".into()),
        "colors": { "add": color("add"), "mod": color("mod"), "del": color("del") },
        "hunks": hunks,
    }).to_string();
    *state.latest_diff.write().await = Some(payload.clone());
    state.send_scroll(payload);
}

/// `md_preview_scroll` 処理
fn handle_scroll(state: &AppState, params: &[rmpv::Value]) {
    // Neovim は整数を符号付き(i64)または符号なし(u64)で送る場合がある
    let line = params.first()
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n.max(1) as u64)))
        .unwrap_or(1);
    let total = params.get(1)
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n.max(1) as u64)))
        .unwrap_or(1);
    let payload = serde_json::json!({
        "type": "scroll", "line": line, "total": total,
    }).to_string();
    state.send_scroll(payload);
}

#[tokio::main]
async fn main() {
    // 引数: <port> [host] [token]
    let args: Vec<String> = std::env::args().collect();
    // プラグイン（Lua 側）がバイナリの版を確かめるのに使う
    if args.get(1).map(String::as_str) == Some("--version") {
        println!("nvim-md-preview {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PORT);
    let host = args.get(2).cloned().unwrap_or_else(|| "127.0.0.1".to_string());
    let token = args.get(3).filter(|t| !t.is_empty()).cloned();
    let lan = !matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1");
    if lan && token.is_none() {
        eprintln!("A token is required to listen on the LAN");
        std::process::exit(2);
    }
    let access = server::Access { lan, token };

    let (tx, _rx) = broadcast::channel::<String>(16);
    let (nvim_tx, nvim_rx) = mpsc::unbounded_channel::<NvimNotify>();
    let state = Arc::new(AppState::new(tx, nvim_tx, access));

    tokio::spawn(run_nvim_writer(nvim_rx));

    {
        let state_server = Arc::clone(&state);
        tokio::spawn(async move {
            server::start_server(&host, port, state_server).await;
        });
    }

    run_rpc_loop(Arc::clone(&state)).await;
}
