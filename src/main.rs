mod markdown;
mod server;

use server::AppState;
use std::{path::PathBuf, sync::Arc};
use tokio::{
    io::AsyncReadExt,
    sync::broadcast,
};

const DEFAULT_PORT: u16 = 9876;

/// Neovim msgpack-rpc の通知メッセージを stdin から読んで処理する。
/// nvim-rs / parity-tokio-ipc に依存せず、全 OS で動作する。
///
/// msgpack-rpc notify フォーマット: [2, method, params]
///   - type=2: notification
///   - method: String
///   - params: Array
async fn run_rpc_loop(state: Arc<AppState>) {
    let mut stdin = tokio::io::stdin();

    loop {
        // msgpack の最初のバイトを読む
        // msgpack fixarray: 0x93 = 3要素配列, 0x94 = 4要素配列
        let mut header = [0u8; 1];
        if stdin.read_exact(&mut header).await.is_err() {
            break; // stdin が閉じた（Neovim 終了）
        }

        // rmpv で Value を読む（先頭バイトを含めて読み直す）
        // stdin をバッファリングして rmpv::decode::read_value を使う
        // 先頭バイトを戻せないので、残りを読んで再構築する
        let msg = match read_msgpack_value(&mut stdin, header[0]).await {
            Ok(v) => v,
            Err(_) => break,
        };

        // [type, method, params] の形式を確認
        let arr = match msg.as_array() {
            Some(a) if a.len() >= 3 => a,
            _ => continue,
        };

        let msg_type = arr[0].as_u64().unwrap_or(0);
        if msg_type != 2 {
            // type=0: request, type=1: response, type=2: notification
            // request/response は現在無視（ping 対応は不要）
            continue;
        }

        let method = match arr[1].as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        let params = match arr[2].as_array() {
            Some(a) => a.clone(),
            None => continue,
        };

        match method.as_str() {
            "md_preview_update" => {
                handle_update(&state, &params).await;
            }
            "md_preview_scroll" => {
                handle_scroll(&state, &params);
            }
            _ => {}
        }
    }
}

/// msgpack の Value を stdin から読む
/// header: 既に読んだ先頭 1 バイト
async fn read_msgpack_value(
    stdin: &mut tokio::io::Stdin,
    first_byte: u8,
) -> Result<rmpv::Value, Box<dyn std::error::Error + Send + Sync>> {
    // 先頭バイトからメッセージ全体のサイズを推定して読み込む
    // msgpack は可変長なので、Cursor を使って段階的に読む
    let mut buf = vec![first_byte];

    // 最大 16MB まで読む（Neovim からの通常のメッセージはずっと小さい）
    const MAX_SIZE: usize = 16 * 1024 * 1024;

    loop {
        let mut cursor = std::io::Cursor::new(&buf);
        match rmpv::decode::read_value(&mut cursor) {
            Ok(val) => {
                // 正常に読めた場合、残りのバイト数を stdin から読み捨てる必要はない
                // cursor.position() が消費したバイト数
                let consumed = cursor.position() as usize;
                // buf に余分なバイトがある場合は stdin を先読みしすぎているので注意
                // ただし通常は buf.len() == consumed になるはず
                let _ = consumed;
                return Ok(val);
            }
            Err(_) => {
                // まだデータが足りない → もっと読む
                if buf.len() >= MAX_SIZE {
                    return Err("message too large".into());
                }
                let mut tmp = [0u8; 4096];
                let n = stdin.read(&mut tmp).await?;
                if n == 0 {
                    return Err("stdin closed".into());
                }
                buf.extend_from_slice(&tmp[..n]);
            }
        }
    }
}

/// `md_preview_update` 処理
async fn handle_update(state: &AppState, params: &[rmpv::Value]) {
    let Some(content) = params.first().and_then(|v| v.as_str()) else { return; };
    let title        = params.get(1).and_then(|v| v.as_str()).unwrap_or("md Preview");
    let base_dir_str = params.get(2).and_then(|v| v.as_str()).unwrap_or("");

    if !base_dir_str.is_empty() {
        *state.base_dir.write().await = Some(PathBuf::from(base_dir_str));
    }

    let (front_matter, body) = markdown::extract_front_matter(content);
    let html = markdown::to_html(&body);

    let payload = serde_json::json!({
        "type": "content",
        "title": title,
        "html": html,
        "front_matter": front_matter,
    }).to_string();

    state.update_html(payload).await;
}

/// `md_preview_scroll` 処理
fn handle_scroll(state: &AppState, params: &[rmpv::Value]) {
    let line  = params.first().and_then(|v| v.as_u64()).unwrap_or(1);
    let total = params.get(1).and_then(|v| v.as_u64()).unwrap_or(1);
    let payload = serde_json::json!({
        "type": "scroll", "line": line, "total": total,
    }).to_string();
    state.send_scroll(payload);
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PORT);

    let (tx, _rx) = broadcast::channel::<String>(16);
    let state = Arc::new(AppState::new(tx));

    {
        let state_server = Arc::clone(&state);
        tokio::spawn(async move {
            server::start_server(port, state_server).await;
        });
    }

    run_rpc_loop(Arc::clone(&state)).await;
}
