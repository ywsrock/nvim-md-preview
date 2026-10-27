use pulldown_cmark::{html, Options, Parser};

/// YAML front matter（`---` で囲まれたブロック）を抽出する。
/// 戻り値: (front_matter, 本文)
pub fn extract_front_matter(content: &str) -> (Option<String>, String) {
    let content = content.replace("\r\n", "\n");

    if !content.starts_with("---\n") {
        return (None, content);
    }

    // 閉じる `\n---\n` を探す
    if let Some(rel) = content[4..].find("\n---\n") {
        let fm = content[4..4 + rel].to_string();
        let body = content[4 + rel + 5..].to_string();
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
