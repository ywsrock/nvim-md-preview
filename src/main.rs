mod markdown;
mod server;

use async_trait::async_trait;
use nvim_rs::{compat::tokio::Compat, create::tokio as create, Handler, Neovim};
use server::AppState;
use std::{path::PathBuf, sync::Arc};
use tokio::{io::Stdout, sync::broadcast};

const DEFAULT_PORT: u16 = 9876;

#[derive(Clone)]
struct MarkdownPreviewHandler {
    state: Arc<AppState>,
}

#[async_trait]
impl Handler for MarkdownPreviewHandler {
    type Writer = Compat<Stdout>;

    async fn handle_notify(
        &self,
        name: String,
        args: Vec<nvim_rs::Value>,
        _nvim: Neovim<Self::Writer>,
    ) {
        match name.as_str() {
            "md_preview_update" => {
                let Some(content) = args.first().and_then(|v| v.as_str()) else { return; };
                let title        = args.get(1).and_then(|v| v.as_str()).unwrap_or("md Preview");
                let base_dir_str = args.get(2).and_then(|v| v.as_str()).unwrap_or("");

                if !base_dir_str.is_empty() {
                    *self.state.base_dir.write().await = Some(PathBuf::from(base_dir_str));
                }

                let (front_matter, body) = markdown::extract_front_matter(content);
                let html = markdown::to_html(&body);

                let payload = serde_json::json!({
                    "type": "content",
                    "title": title,
                    "html": html,
                    "front_matter": front_matter,
                }).to_string();

                self.state.update_html(payload).await;
            }
            "md_preview_scroll" => {
                let line  = args.first().and_then(|v| v.as_u64()).unwrap_or(1);
                let total = args.get(1).and_then(|v| v.as_u64()).unwrap_or(1);
                let payload = serde_json::json!({
                    "type": "scroll", "line": line, "total": total,
                }).to_string();
                self.state.send_scroll(payload);
            }
            _ => {}
        }
    }

    async fn handle_request(
        &self,
        name: String,
        _args: Vec<nvim_rs::Value>,
        _nvim: Neovim<Self::Writer>,
    ) -> Result<nvim_rs::Value, nvim_rs::Value> {
        match name.as_str() {
            "md_preview_ping" => Ok(nvim_rs::Value::from("pong")),
            _ => Err(nvim_rs::Value::from(format!("Unknown request: {}", name))),
        }
    }
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

    let handler = MarkdownPreviewHandler { state: Arc::clone(&state) };
    let (_nvim, io_handler) = create::new_parent(handler).await;
    let _ = io_handler.await;
}
