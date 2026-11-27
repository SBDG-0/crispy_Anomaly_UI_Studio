use crate::document::Document;
pub fn format(source: &str) -> Result<String, String> {
    let doc = Document::parse(source.into(), false, false)?;
    let nl = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = String::new();
    fn gap(text: &str, depth: usize, out: &mut String, nl: &str) {
        let text = text.trim();
        if !text.is_empty() {
            for line in text.lines() {
                out.push_str(&"    ".repeat(depth));
                out.push_str(line.trim_end());
                out.push_str(nl);
            }
        }
    }
    fn node(doc: &Document, index: usize, depth: usize, out: &mut String, nl: &str) {
        let n = &doc.nodes[index];
        let children: Vec<_> = doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.parent == Some(index))
            .map(|(i, _)| i)
            .collect();
        let raw = &doc.source[n.span.clone()];
        if children.is_empty() || !n.text.trim().is_empty() {
            out.push_str(&"    ".repeat(depth));
            out.push_str(raw);
            out.push_str(nl);
            return;
        }
        out.push_str(&"    ".repeat(depth));
        out.push_str(&doc.source[n.span.start..n.open_end]);
        out.push_str(nl);
        let mut cursor = n.open_end;
        for child in children {
            let c = &doc.nodes[child];
            gap(&doc.source[cursor..c.span.start], depth + 1, out, nl);
            node(doc, child, depth + 1, out, nl);
            cursor = c.span.end;
        }
        let close = doc.source[n.open_end..n.span.end]
            .rfind("</")
            .map(|i| i + n.open_end)
            .unwrap_or(n.span.end);
        gap(&doc.source[cursor..close], depth + 1, out, nl);
        out.push_str(&"    ".repeat(depth));
        out.push_str(&doc.source[close..n.span.end]);
        out.push_str(nl);
    }
    let mut cursor = 0;
    for (i, n) in doc
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.parent.is_none())
    {
        gap(&source[cursor..n.span.start], 0, &mut result, nl);
        node(&doc, i, 0, &mut result, nl);
        cursor = n.span.end;
    }
    gap(&source[cursor..], 0, &mut result, nl);
    Document::parse(result.clone(), false, false)?;
    Ok(result)
}
pub fn decode(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(end) = rest.find(';') {
            let value = &rest[1..end];
            let c = match value {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => value
                    .strip_prefix("#x")
                    .and_then(|v| u32::from_str_radix(v, 16).ok())
                    .or_else(|| value.strip_prefix('#').and_then(|v| v.parse().ok()))
                    .and_then(char::from_u32),
            };
            if let Some(c) = c {
                out.push(c);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}
pub fn escape(s: &str, attribute: bool) -> String {
    let s = s
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    if attribute {
        s.replace('"', "&quot;").replace('\'', "&apos;")
    } else {
        s
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatting_keeps_values_comments_and_includes() {
        let s = "<w><a x='1'/><!-- keep --><b>  exact &amp; text  </b></w>";
        let out = format(s).unwrap();
        assert!(out.contains("    <a x='1'/>"));
        assert!(out.contains("<!-- keep -->"));
        assert!(out.contains(">  exact &amp; text  </b>"));
        assert_eq!(decode("&lt;é &#x4E2D; &amp;"), "<é 中 &");
    }
    #[test]
    fn mixed_content_not_rewritten() {
        assert!(format("<a>before <b/> after</a>")
            .unwrap()
            .starts_with("<a>before <b/> after</a>"));
    }
}
