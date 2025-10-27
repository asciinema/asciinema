pub fn extract_asciicast_link(html: &str) -> Option<String> {
    let html_lc = html.to_ascii_lowercase();
    let head_start = html_lc.find("<head")?;
    let head_end = html_lc[head_start..].find("</head>")? + head_start;
    let head = &html[head_start..head_end];
    let head_lc = head.to_ascii_lowercase();
    let mut head_offset = 0;

    while let Some(link_pos) = head_lc[head_offset..].find("<link") {
        let link_start = head_offset + link_pos;
        let link_end = head[link_start..].find('>')? + link_start + 1;
        let link = &head[link_start..link_end];
        head_offset = link_end;

        if let Some(rel) = attr(link, "rel") {
            if rel
                .split_whitespace()
                .any(|t| t.eq_ignore_ascii_case("alternate"))
            {
                if let Some(t) = attr(link, "type") {
                    if t.eq_ignore_ascii_case("application/x-asciicast")
                        || t.eq_ignore_ascii_case("application/asciicast+json")
                    {
                        if let Some(href) = attr(link, "href") {
                            if !href.is_empty() {
                                return Some(href.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let tag_lc = tag.to_ascii_lowercase();
    let prefix = format!("{}=", name.to_ascii_lowercase());
    let mut i = tag_lc.find(&prefix)? + prefix.len();
    let bytes = tag.as_bytes();

    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    if i >= bytes.len() {
        return None;
    }

    let quote = bytes[i];

    if quote == b'\'' || quote == b'"' {
        let start = i + 1;
        let end = tag[start..].find(quote as char)? + start;

        Some(&tag[start..end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_asciicast_link_valid_html() {
        let html = r#"
        <html>
        <head>
            <title>Test</title>
            <link rel="alternate" type="application/x-asciicast" href="https://example.com/demo.cast">
        </head>
        <body>Content</body>
        </html>
        "#;

        assert_eq!(
            extract_asciicast_link(html),
            Some("https://example.com/demo.cast".to_string())
        );
    }

    #[test]
    fn extract_asciicast_link_alternate_mime_type() {
        let html = r#"
        <html>
        <head>
            <link rel="alternate" type="application/asciicast+json" href="/demo.json">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), Some("/demo.json".to_string()));
    }

    #[test]
    fn extract_asciicast_link_multiple_rel_values() {
        let html = r#"
        <html>
        <head>
            <link rel="foobar alternate" type="application/x-asciicast" href="demo.cast">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), Some("demo.cast".to_string()));
    }

    #[test]
    fn extract_asciicast_link_case_insensitive() {
        let html = r#"
        <HTML>
        <HEAD>
            <LINK REL="ALTERNATE" TYPE="APPLICATION/X-ASCIICAST" HREF="DEMO.CAST">
        </HEAD>
        </HTML>
        "#;

        assert_eq!(extract_asciicast_link(html), Some("DEMO.CAST".to_string()));
    }

    #[test]
    fn extract_asciicast_link_single_quotes() {
        let html = r#"
        <html>
        <head>
            <link rel='alternate' type='application/x-asciicast' href='demo.cast'>
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), Some("demo.cast".to_string()));
    }

    #[test]
    fn extract_asciicast_link_mixed_quotes() {
        let html = r#"
        <html>
        <head>
            <link rel="alternate" type='application/x-asciicast' href="demo.cast">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), Some("demo.cast".to_string()));
    }

    #[test]
    fn extract_asciicast_link_multiple_links() {
        let html = r#"
        <html>
        <head>
            <link rel="stylesheet" href="style.css">
            <link rel="alternate" type="application/rss+xml" href="feed.rss">
            <link rel="alternate" type="application/x-asciicast" href="first.cast">
            <link rel="alternate" type="application/x-asciicast" href="second.cast">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), Some("first.cast".to_string()));
    }

    #[test]
    fn extract_asciicast_link_no_head() {
        let html = r#"
        <html>
        <body>
            <link rel="alternate" type="application/x-asciicast" href="demo.cast">
        </body>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_no_matching_link() {
        let html = r#"
        <html>
        <head>
            <link rel="stylesheet" href="style.css">
            <link rel="alternate" type="application/rss+xml" href="feed.rss">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_wrong_rel() {
        let html = r#"
        <html>
        <head>
            <link rel="stylesheet" type="application/x-asciicast" href="demo.cast">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_wrong_type() {
        let html = r#"
        <html>
        <head>
            <link rel="alternate" type="text/plain" href="demo.cast">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_no_href() {
        let html = r#"
        <html>
        <head>
            <link rel="alternate" type="application/x-asciicast">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_empty_href() {
        let html = r#"
        <html>
        <head>
            <link rel="alternate" type="application/x-asciicast" href="">
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_malformed_html() {
        let html = r#"
        <html>
        <head>
            <link rel="alternate" type="application/x-asciicast" href="demo.cast"
        </head>
        </html>
        "#;

        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_empty_html() {
        assert_eq!(extract_asciicast_link(""), None);
    }

    #[test]
    fn extract_asciicast_link_invalid_html() {
        let html = "This is not HTML at all";
        assert_eq!(extract_asciicast_link(html), None);
    }

    #[test]
    fn extract_asciicast_link_special_characters_in_href() {
        let html = r#"
        <html>
        <head>
            <link rel="alternate" type="application/x-asciicast" href="https://example.com/cast?id=123&format=v3">
        </head>
        </html>
        "#;

        assert_eq!(
            extract_asciicast_link(html),
            Some("https://example.com/cast?id=123&format=v3".to_string())
        );
    }
}
