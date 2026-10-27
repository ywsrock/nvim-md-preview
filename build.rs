// build.rs — cargo build 前にライブラリファイルの存在を確認する
use std::path::Path;

fn main() {
    let required = [
        "src/embedded_lib/highlight.min.js.gz",
        "src/embedded_lib/hljs-dark.min.css.gz",
        "src/embedded_lib/hljs-light.min.css.gz",
        "src/embedded_lib/mermaid.min.js.gz",
        "src/embedded_lib/katex.min.js.gz",
        "src/embedded_lib/katex.min.css.gz",
        "src/embedded_lib/auto-render.min.js.gz",
    ];

    for file in &required {
        if !Path::new(file).exists() {
            panic!(
                "\n\n[nvim-md-preview build error]\nMissing: {}\n\nPlease run first:\n  bash download-libs.sh\n",
                file
            );
        }
        println!("cargo:rerun-if-changed={}", file);
    }

    let fonts_dir = Path::new("src/embedded_lib/fonts");
    let is_empty = fonts_dir
        .read_dir()
        .map(|mut d| d.next().is_none())
        .unwrap_or(true);
    if !fonts_dir.is_dir() || is_empty {
        panic!(
            "\n\n[nvim-md-preview build error]\nMissing KaTeX fonts in src/embedded_lib/fonts/\n\nPlease run first:\n  bash download-libs.sh\n"
        );
    }
    println!("cargo:rerun-if-changed=src/embedded_lib/fonts");
}
