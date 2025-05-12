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
                            return Some(href.to_string());
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
        let start = i;
        let mut end = i;

        while end < bytes.len()
            && !bytes[end].is_ascii_whitespace()
            && bytes[end] != b'>'
            && bytes[end] != b'/'
        {
            end += 1;
        }

        Some(&tag[start..end])
    }
}
