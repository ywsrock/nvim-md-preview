use pulldown_cmark::{html, Options, Parser};

/// YAML front matter（`---` で囲まれたブロック）を抽出する。
/// 戻り値: (front_matter, 本文)
pub fn extract_front_matter(content: &str) -> (Option<String>, String) {
    let content = content.replace("\r\n", "\n");

    if !content.starts_with("---\n") {
        return (None, content);
    }

    // content[3..] = "\n..." の中で "\n---\n" を探す
    // これにより "---\n---\n" （空 front matter）も検出できる
    let search_from = &content[3..];
    if let Some(pos) = search_from.find("\n---\n") {
        let fm_end   = 4 + pos;            // content[4..fm_end] が front matter
        let body_start = 3 + pos + 5;      // "\n---\n" = 5 文字
        let fm   = content[4..fm_end].trim_end_matches('\n').to_string();
        let body = if body_start <= content.len() {
            content[body_start..].to_string()
        } else {
            // body_start がファイル末尾を超える場合（front matter が最後まで）
            String::new()
        };
        return (Some(fm), body);
    }

    // ファイル末尾が `\n---` で終わる場合
    if content.ends_with("\n---") {
        let body_end = content.len() - 4;
        let fm = content[4..body_end].to_string();
        return (Some(fm), String::new());
    }

    (None, content)
}

/// Markdown テキストを HTML 文字列に変換する
pub fn to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

