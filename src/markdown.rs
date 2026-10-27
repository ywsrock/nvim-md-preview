use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};

/// 描画オプション（Neovim の設定から渡される）
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderOptions {
    /// `[[page]]` / `[[page|label]]` 形式の Wiki リンクを有効にする
    pub wiki_links: bool,
}

/// テキスト中の `[[target|label]]` を <a class="wiki-link"> に展開する。
/// target に拡張子が無ければ `.md` を補う。戻り値は Text / Html イベントの列
fn expand_wiki_links<'a>(text: &str) -> Vec<Event<'a>> {
    let mut events = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let Some(len) = rest[start + 2..].find("]]") else { break; };
        let inner = &rest[start + 2..start + 2 + len];
        // 空や改行を含むものはリンク扱いしない
        if inner.trim().is_empty() || inner.contains('\n') {
            events.push(Event::Text(CowStr::from(rest[..start + 2].to_string())));
            rest = &rest[start + 2..];
            continue;
        }
        if start > 0 {
            events.push(Event::Text(CowStr::from(rest[..start].to_string())));
        }
        let (target, label) = match inner.split_once('|') {
            Some((t, l)) => (t.trim(), l.trim()),
            None => (inner.trim(), inner.trim()),
        };
        let has_ext = std::path::Path::new(target).extension().is_some();
        let href = if has_ext || target.starts_with('#') { target.to_string() } else { format!("{}.md", target) };
        events.push(Event::Html(CowStr::from(format!(
            r#"<a class="wiki-link" href="{}">{}</a>"#,
            html_escape(&href),
            html_escape(label),
        ))));
        rest = &rest[start + 2 + len + 2..];
    }
    if !rest.is_empty() {
        events.push(Event::Text(CowStr::from(rest.to_string())));
    }
    events
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

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

/// front matter の行数（本文の行番号をバッファの行番号に戻すためのオフセット）
/// `---\n<fm>\n---\n` なので fm の行数 + 2
pub fn front_matter_line_count(front_matter: Option<&str>) -> usize {
    match front_matter {
        Some(fm) => fm.lines().count() + 2,
        None => 0,
    }
}

/// Markdown テキストを HTML 文字列に変換する
///
/// スクロール同期・クリックジャンプ用に、ブロック要素の直前（リスト項目と引用は直後）に
/// `<span class="src-line" data-line="N"></span>` を埋め込む。N は 1 始まりの行番号で、
/// `line_offset` を足して返す（front matter 分のずれを補正するため）。
pub fn to_html(markdown: &str, line_offset: usize, opts: RenderOptions) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // バイトオフセット → 行番号 の変換表（各行の開始オフセット）
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(markdown.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let line_of = |offset: usize| -> usize {
        // offset 以下の最大の開始位置を二分探索
        let idx = match line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        idx + 1 + line_offset
    };

    let marker = |offset: usize| -> Event<'static> {
        Event::Html(format!(r#"<span class="src-line" data-line="{}"></span>"#, line_of(offset)).into())
    };

    let mut in_table = 0usize;
    let mut in_code = false;
    let mut events: Vec<Event> = Vec::new();
    // pulldown-cmark は "[" をリンク候補として別のテキストに分けるので、
    // 連続するテキストをまとめてから Wiki リンクを展開する
    let mut pending_text = String::new();
    for (event, range) in Parser::new_ext(markdown, options).into_offset_iter() {
        if opts.wiki_links && !in_code {
            if let Event::Text(text) = &event {
                pending_text.push_str(text);
                continue;
            }
            if !pending_text.is_empty() {
                events.extend(expand_wiki_links(&pending_text));
                pending_text.clear();
            }
        }
        match &event {
            Event::Start(Tag::Table(_)) => { in_table += 1; events.push(event); }
            Event::End(TagEnd::Table) => { in_table -= 1; events.push(event); }
            Event::End(TagEnd::CodeBlock) => { in_code = false; events.push(event); }
            // テーブル内（<tr>/<td> の間）に span を置くと HTML が壊れるので入れない
            _ if in_table > 0 => events.push(event),
            // <li>/<blockquote> は中に置いてよい（直前に置くと <ul> 直下に span が入る）
            Event::Start(Tag::Item) | Event::Start(Tag::BlockQuote(_)) => {
                events.push(event);
                events.push(marker(range.start));
            }
            Event::Start(Tag::Paragraph)
            | Event::Start(Tag::Heading { .. })
            | Event::Start(Tag::HtmlBlock)
            | Event::Start(Tag::List(_)) => {
                events.push(marker(range.start));
                events.push(event);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                events.push(marker(range.start));
                events.push(event);
            }
            Event::Rule => {
                events.push(marker(range.start));
                events.push(event);
            }
            _ => events.push(event),
        }
    }

    if !pending_text.is_empty() {
        events.extend(expand_wiki_links(&pending_text));
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());
    html_output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_is_split_from_body() {
        let (fm, body) = extract_front_matter("---\ntitle: x\n---\n# Hi\n");
        assert_eq!(fm.as_deref(), Some("title: x"));
        assert_eq!(body, "# Hi\n");
        assert_eq!(front_matter_line_count(fm.as_deref()), 3);
    }

    #[test]
    fn no_front_matter() {
        let (fm, body) = extract_front_matter("# Hi\n");
        assert!(fm.is_none());
        assert_eq!(body, "# Hi\n");
        assert_eq!(front_matter_line_count(None), 0);
    }

    #[test]
    fn empty_front_matter() {
        let (fm, body) = extract_front_matter("---\n---\nbody");
        assert_eq!(fm.as_deref(), Some(""));
        assert_eq!(body, "body");
        assert_eq!(front_matter_line_count(fm.as_deref()), 2);
    }

    #[test]
    fn crlf_is_normalized() {
        let (fm, body) = extract_front_matter("---\r\na: 1\r\n---\r\nx\r\n");
        assert_eq!(fm.as_deref(), Some("a: 1"));
        assert_eq!(body, "x\n");
    }

    #[test]
    fn line_markers_point_to_source_lines() {
        let md = "# Title\n\npara one\n\n- a\n- b\n\n```rs\nlet x = 1;\n```\n";
        let html = to_html(md, 0, RenderOptions::default());
        let lines: Vec<&str> = html
            .match_indices("data-line=\"")
            .map(|(i, _)| {
                let rest = &html[i + 11..];
                &rest[..rest.find('"').unwrap()]
            })
            .collect();
        // 見出し(1), 段落(3), リスト(5), 項目a(5), 項目b(6), コード(8)
        assert_eq!(lines, vec!["1", "3", "5", "5", "6", "8"]);
    }

    #[test]
    fn line_offset_shifts_markers() {
        let html = to_html("para\n", 3, RenderOptions::default());
        assert!(html.contains(r#"data-line="4""#));
    }

    #[test]
    fn no_markers_inside_tables() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let html = to_html(md, 0, RenderOptions::default());
        assert!(!html.contains("src-line"), "{}", html);
    }

    #[test]
    fn wiki_links_are_expanded() {
        let opts = RenderOptions { wiki_links: true };
        let html = to_html("see [[My Page]] and [[docs/a.md|A]] but not `[[code]]`\n", 0, opts);
        assert!(html.contains(r#"<a class="wiki-link" href="My Page.md">My Page</a>"#), "{}", html);
        assert!(html.contains(r#"<a class="wiki-link" href="docs/a.md">A</a>"#), "{}", html);
        assert!(html.contains("<code>[[code]]</code>"), "{}", html);
    }

    #[test]
    fn wiki_links_disabled_by_default() {
        let html = to_html("[[x]]\n", 0, RenderOptions::default());
        assert!(!html.contains("wiki-link"), "{}", html);
    }

    #[test]
    fn wiki_links_skip_code_blocks_and_unclosed() {
        let opts = RenderOptions { wiki_links: true };
        let html = to_html("```\n[[x]]\n```\n\n[[unclosed\n", 0, opts);
        assert!(!html.contains("wiki-link"), "{}", html);
        assert!(html.contains("[[unclosed"), "{}", html);
    }

    #[test]
    fn list_item_marker_is_inside_li() {
        let html = to_html("- item\n", 0, RenderOptions::default());
        assert!(html.contains(r#"<li><span class="src-line" data-line="1"></span>item</li>"#), "{}", html);
    }
}
