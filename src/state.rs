//! サーバーの状態と設定の型（Neovim から届いた内容・設定、アクセス制御）

use std::{path::PathBuf, sync::Arc};
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
    /// 表示の仕方: "markdown"（Markdown として描画）| "code"（ハイライト付きのコード）
    pub view_kind: Arc<RwLock<String>>,
    /// Neovim への RPC 通知チャネル（stdout に書く）
    pub nvim_tx: mpsc::UnboundedSender<NvimNotify>,
    /// Markdown 描画オプション
    pub render_opts: Arc<RwLock<crate::markdown::RenderOptions>>,
    /// エクスポート設定
    pub export_opts: Arc<RwLock<ExportOptions>>,
    /// 初期テーマ: "dark" | "light" | "auto"（OS 設定に従う）
    pub theme: Arc<RwLock<String>>,
    /// ユーザー CSS（/custom.css で配信）
    pub custom_css: Arc<RwLock<String>>,
    /// ブラウザのキーボードショートカット（t / / / Ctrl+F / p）を使う
    pub browser_keys: Arc<RwLock<bool>>,
    /// Export メニューに出す形式（export.formats）
    pub export_formats: Arc<RwLock<Vec<String>>>,
    /// アクセス制御（起動時に決まり、以後変わらない）
    pub access: Access,
    /// 最新の差分表示メッセージ（後から接続したブラウザにも送る）
    pub latest_diff: Arc<RwLock<Option<String>>>,
    /// 実際に待ち受けているポート（PDF をブラウザで印刷するときに /print を開くのに使う）
    pub port: Arc<std::sync::atomic::AtomicU16>,
    /// PDF の印刷に使う Chromium 系ブラウザ（Neovim 側で探して送ってくる。無ければ Typst を使う）
    pub browser_path: Arc<RwLock<Option<PathBuf>>>,
    /// Typst のパッケージ（cmarker）を置いたディレクトリ。あればネットワークに取りに行かない
    pub typst_packages: Arc<RwLock<Option<PathBuf>>>,
    /// plantuml コマンド（または plantuml.jar）のパス。無ければローカルでは描かない
    pub plantuml_path: Arc<RwLock<Option<PathBuf>>>,
    /// PlantUML サーバーの URL。設定されたときだけサーバーで描く（空なら使わない）
    pub plantuml_server: Arc<RwLock<String>>,
    /// PlantUML の描画結果のキャッシュ
    pub plantuml_cache: Arc<crate::plantuml::PlantumlCache>,
}

/// サーバーへのアクセス制御
#[derive(Clone, Debug, Default)]
pub struct Access {
    /// LAN に公開している（0.0.0.0 などに bind）。false なら localhost 以外の Host を拒否する
    pub lan: bool,
    /// 設定されていれば全リクエストに必要（?token= か Cookie）
    pub token: Option<String>,
    /// このプレビューの ID（Neovim が環境変数 MD_PREVIEW_INSTANCE で渡す）。
    /// ブラウザで開く URL に ?instance= として付け、別のプレビューのサーバーにつながったことを見分ける
    pub instance: Option<String>,
}

/// エクスポート設定（PDF は Typst、それ以外は pandoc に反映する）
#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// PDF の作り方: "browser"（プレビューをブラウザで印刷）| "typst"
    pub pdf_engine: String,
    /// 用紙サイズ（Typst の paper 名: "a4", "us-letter" など）
    pub paper: String,
    /// 余白（Typst の長さ: "2cm" など）
    pub margin: String,
    /// 本文フォント（空なら既定のフォント列）
    pub font: String,
    /// 本文サイズ（pt）
    pub font_size: f64,
    /// 目次を入れる
    pub toc: bool,
    /// 見出しに番号を付ける
    pub numbering: bool,
    /// ユーザーの Typst テンプレート（指定時は既定のスタイルの代わりに使う）
    pub template: Option<PathBuf>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            pdf_engine: "browser".to_string(),
            paper: "a4".to_string(),
            margin: "2cm".to_string(),
            font: String::new(),
            font_size: 11.0,
            toc: false,
            numbering: false,
            template: None,
        }
    }
}

impl AppState {
    pub fn new(tx: HtmlSender, nvim_tx: mpsc::UnboundedSender<NvimNotify>, access: Access) -> Self {
        Self {
            tx,
            latest_html: Arc::new(RwLock::new(None)),
            base_dir: Arc::new(RwLock::new(None)),
            current_file: Arc::new(RwLock::new(None)),
            pandoc_path: Arc::new(RwLock::new(None)),
            typst_path: Arc::new(RwLock::new(None)),
            current_content: Arc::new(RwLock::new(None)),
            view_kind: Arc::new(RwLock::new("markdown".to_string())),
            nvim_tx,
            render_opts: Arc::new(RwLock::new(Default::default())),
            export_opts: Arc::new(RwLock::new(ExportOptions::default())),
            theme: Arc::new(RwLock::new("auto".to_string())),
            custom_css: Arc::new(RwLock::new(String::new())),
            browser_keys: Arc::new(RwLock::new(true)),
            export_formats: Arc::new(RwLock::new(
                ["pdf", "docx", "html", "epub", "markdown"].iter().map(|s| s.to_string()).collect(),
            )),
            access,
            latest_diff: Arc::new(RwLock::new(None)),
            port: Arc::new(std::sync::atomic::AtomicU16::new(0)),
            browser_path: Arc::new(RwLock::new(None)),
            typst_packages: Arc::new(RwLock::new(None)),
            plantuml_path: Arc::new(RwLock::new(None)),
            plantuml_server: Arc::new(RwLock::new(String::new())),
            plantuml_cache: Arc::new(Default::default()),
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
