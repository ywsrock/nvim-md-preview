//! PlantUML の図を描く（/plantuml）
//!   - plantuml_server が設定されていれば、そのサーバーの SVG の URL を返す（ブラウザが直接取りに行く）
//!   - そうでなければローカルの plantuml コマンド（または .jar）で SVG を作って返す

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command, sync::Mutex};
use crate::state::AppState;

/// 1 つの図の描画にかける時間の上限
const TIMEOUT: Duration = Duration::from_secs(30);
/// キャッシュする図の数の上限（超えたら空にする）
const CACHE_MAX: usize = 200;

/// 描画結果のキャッシュ（ソース → SVG）と、同時に動かす plantuml を 1 つに絞るためのロック
#[derive(Default)]
pub struct PlantumlCache {
    svgs: Mutex<HashMap<String, String>>,
}

/// POST /plantuml — 本文は図のソース
/// 応答: {"url": "..."} | {"svg": "..."} | {"error": "..."}
pub(crate) async fn plantuml_handler(State(state): State<Arc<AppState>>, body: String) -> Response {
    let server = state.plantuml_server.read().await.clone();
    let result = if !server.is_empty() {
        serde_json::json!({ "url": server_url(&server, &body) })
    } else {
        let path = state.plantuml_path.read().await.clone();
        match path {
            None => serde_json::json!({
                "error": "PlantUML is not installed. Install it, or set plantuml_server in setup()."
            }),
            Some(path) => match render_local(&state, &path, &body).await {
                Ok(svg) => serde_json::json!({ "svg": svg }),
                Err(e) => serde_json::json!({ "error": e }),
            },
        }
    };
    ([(header::CONTENT_TYPE, "application/json")], result.to_string()).into_response()
}

/// @startuml / @enduml などが無ければ補う
fn wrap_source(source: &str) -> String {
    if source.trim_start().starts_with("@start") {
        source.to_string()
    } else {
        format!("@startuml\n{}\n@enduml\n", source.trim_end())
    }
}

/// ローカルの plantuml で SVG を作る（同じソースはキャッシュから返す）
async fn render_local(state: &AppState, path: &Path, source: &str) -> Result<String, String> {
    let source = wrap_source(source);
    // ロックを持ったまま描画して、plantuml（JVM）を同時に何個も起動しないようにする
    let mut cache = state.plantuml_cache.svgs.lock().await;
    if let Some(svg) = cache.get(&source) {
        return Ok(svg.clone());
    }
    let svg = run_plantuml(path, &source).await?;
    if cache.len() >= CACHE_MAX {
        cache.clear();
    }
    cache.insert(source, svg.clone());
    Ok(svg)
}

async fn run_plantuml(path: &Path, source: &str) -> Result<String, String> {
    let is_jar = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("jar"));
    let mut cmd = if is_jar {
        let mut c = Command::new("java");
        c.arg("-Djava.awt.headless=true").arg("-jar").arg(path);
        c
    } else {
        Command::new(path)
    };
    cmd.args(["-tsvg", "-pipe", "-charset", "UTF-8"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| format!("Cannot run {}: {}", path.display(), e))?;
    let mut stdin = child.stdin.take().ok_or("Cannot write to plantuml")?;
    let input = source.as_bytes().to_vec();
    tokio::spawn(async move {
        let _ = stdin.write_all(&input).await;
    });

    let out = tokio::time::timeout(TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| format!("plantuml did not finish in {} seconds", TIMEOUT.as_secs()))?
        .map_err(|e| format!("plantuml failed: {}", e))?;

    // 文法エラーのときも plantuml はエラーの内容を描いた SVG を返す（終了コードは 0 以外）
    let stdout = String::from_utf8_lossy(&out.stdout);
    match stdout.find("<svg") {
        Some(start) => Ok(stdout[start..].trim_end().to_string()),
        None => {
            let err = String::from_utf8_lossy(&out.stderr);
            let line = err.lines().find(|l| !l.trim().is_empty()).unwrap_or("no output");
            Err(format!("plantuml failed: {}", line.trim()))
        }
    }
}

/// PlantUML サーバーの SVG の URL（{server}/svg/{圧縮して符号化したソース}）
pub(crate) fn server_url(server: &str, source: &str) -> String {
    format!("{}/svg/{}", server.trim_end_matches('/'), encode(&wrap_source(source)))
}

/// PlantUML のテキスト符号化: deflate で圧縮し、PlantUML 独自の 64 文字で表す
pub(crate) fn encode(text: &str) -> String {
    const ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";
    let data = miniz_oxide::deflate::compress_to_vec(text.as_bytes(), 9);
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            out.push(ALPHABET[(n >> (18 - 6 * i) & 0x3f) as usize] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(s: &str) -> String {
        const ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";
        let mut bytes = Vec::new();
        for chunk in s.as_bytes().chunks(4) {
            let n = chunk.iter().fold(0u32, |acc, c| {
                acc << 6 | ALPHABET.iter().position(|a| a == c).unwrap() as u32
            });
            bytes.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8]);
        }
        // 末尾の詰め物（0）は deflate の終端より後ろなので無視される
        let raw = miniz_oxide::inflate::decompress_to_vec(&bytes).unwrap();
        String::from_utf8(raw).unwrap()
    }

    #[test]
    fn encode_round_trips() {
        for text in ["Bob -> Alice : hello", "@startuml\nA -> B : 日本語\n@enduml\n", "x"] {
            assert_eq!(decode(&encode(text)), text);
        }
    }

    #[test]
    fn server_url_wraps_source() {
        let url = server_url("https://www.plantuml.com/plantuml/", "A -> B");
        let encoded = url.strip_prefix("https://www.plantuml.com/plantuml/svg/").unwrap();
        assert_eq!(decode(encoded), "@startuml\nA -> B\n@enduml\n");
    }

    #[test]
    fn wrap_keeps_explicit_start() {
        assert_eq!(wrap_source("@startmindmap\n* a\n@endmindmap"), "@startmindmap\n* a\n@endmindmap");
    }
}
