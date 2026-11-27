use std::ops::Range;
pub struct Edit {
    pub text: String,
    pub primary: usize,
    pub secondary: usize,
}
fn byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map(|x| x.0).unwrap_or(s.len())
}
fn mapped(pos: usize, ops: &[(Range<usize>, String)]) -> usize {
    let mut delta = 0isize;
    for (r, v) in ops {
        if pos < r.start {
            break;
        }
        if pos <= r.end {
            return (r.start as isize + delta + v.len() as isize) as usize;
        }
        delta += v.len() as isize - (r.end - r.start) as isize;
    }
    (pos as isize + delta) as usize
}
/// Whole-line toggle semantics, with indentation shared across the selection.
pub fn toggle(
    source: &str,
    primary: usize,
    secondary: usize,
    extension: &str,
) -> Result<Edit, String> {
    toggle_mode(source, primary, secondary, extension, None)
}
pub fn toggle_mode(
    source: &str,
    primary: usize,
    secondary: usize,
    extension: &str,
    force: Option<bool>,
) -> Result<Edit, String> {
    let p = byte(source, primary);
    let q = byte(source, secondary);
    let lo = p.min(q);
    let hi = p.max(q);
    let start = source[..lo].rfind('\n').map(|n| n + 1).unwrap_or(0);
    let adjusted = if hi > lo && hi > 0 && source.as_bytes()[hi - 1] == b'\n' {
        hi - 1
    } else {
        hi
    };
    let end = source[adjusted..]
        .find('\n')
        .map(|n| adjusted + n)
        .unwrap_or(source.len());
    let mut lines = vec![];
    let mut offset = start;
    for line in source[start..end].split('\n') {
        let line = line.trim_end_matches('\r');
        lines.push((offset, line));
        offset += source[offset..]
            .find('\n')
            .map(|n| n + 1)
            .unwrap_or(line.len());
    }
    let mut ops: Vec<(Range<usize>, String)> = vec![];
    if extension.eq_ignore_ascii_case("xml") {
        let raw = &source[start..end];
        let a = raw.len() - raw.trim_start().len();
        let b = raw.trim_end().len().max(a);
        let content = &raw[a..b];
        if content.starts_with("<!--") && content.ends_with("-->") {
            let mut head = 4;
            let mut tail = 3;
            if content[head..].starts_with(' ') {
                head += 1
            }
            if content[..content.len() - tail].ends_with(' ') {
                tail += 1
            }
            ops.push((start + a..start + a + head, String::new()));
            ops.push((start + b - tail..start + b, String::new()));
        } else {
            if content.contains("--") {
                return Err("XML comments cannot contain '--' or nested comments. Select a block without existing comment markers.".into());
            }
            ops.push((start + a..start + a, "<!-- ".into()));
            ops.push((start + b..start + b, " -->".into()));
        }
    } else {
        let token = if extension.eq_ignore_ascii_case("script") {
            "--"
        } else {
            ";"
        };
        let only_blank = lines.iter().all(|(_, s)| s.trim().is_empty());
        let relevant: Vec<_> = lines
            .iter()
            .filter(|(_, s)| only_blank || !s.trim().is_empty())
            .collect();
        let mut remove = !only_blank
            && relevant
                .iter()
                .all(|(_, s)| s.trim_start().starts_with(token));
        if let Some(forced) = force {
            remove = forced;
        }
        let min_column = relevant
            .iter()
            .map(|(_, s)| {
                let mut col = 0;
                for c in s.chars().take_while(|c| *c == ' ' || *c == '\t') {
                    col += if c == '\t' { 4 - col % 4 } else { 1 };
                }
                col
            })
            .min()
            .unwrap_or(0)
            / 4
            * 4;
        for &&(offset, line) in &relevant {
            let indent = line.len() - line.trim_start().len();
            if remove {
                let begin = offset + indent;
                let mut len = token.len();
                if line[indent + len..].starts_with(' ') {
                    len += 1
                }
                ops.push((begin..begin + len, String::new()));
            } else {
                let (mut col, mut index) = (0, 0);
                for (i, c) in line[..indent].char_indices() {
                    let next = col + if c == '\t' { 4 - col % 4 } else { 1 };
                    if next > min_column {
                        break;
                    }
                    col = next;
                    index = i + c.len_utf8();
                }
                ops.push((offset + index..offset + index, format!("{token} ")));
            }
        }
    }
    ops.sort_by_key(|(r, _)| r.start);
    let np = mapped(p, &ops);
    let nq = mapped(q, &ops);
    let mut text = source.to_owned();
    for (r, v) in ops.into_iter().rev() {
        text.replace_range(r, &v)
    }
    Ok(Edit {
        primary: text[..np.min(text.len())].chars().count(),
        secondary: text[..nq.min(text.len())].chars().count(),
        text,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggles_selection_preserving_indent_crlf_and_unicode() {
        let s = "    foo=Привет\r\n        bar=2\r\nnext=3";
        let e = toggle(s, 0, s.find("next").unwrap() - 6, "ltx").unwrap();
        assert!(e.text.starts_with("    ; foo=Привет\r\n    ;     bar=2"));
        let back = toggle(&e.text, e.primary, e.secondary, "ltx").unwrap();
        assert_eq!(back.text, s);
    }
    #[test]
    fn excludes_line_after_selection_ending_at_column_zero() {
        let e = toggle("one\ntwo\nthree", 0, 4, "ltx").unwrap();
        assert_eq!(e.text, "; one\ntwo\nthree");
    }
    #[test]
    fn mixed_comments_adds_and_all_comments_removes() {
        let e = toggle("; a\nb", 0, 5, "ltx").unwrap();
        assert_eq!(e.text, "; ; a\n; b");
        let back = toggle(&e.text, e.primary, e.secondary, "ltx").unwrap();
        assert_eq!(back.text, "; a\nb");
    }
    #[test]
    fn empty_line_and_script() {
        assert_eq!(toggle("", 0, 0, "ltx").unwrap().text, "; ");
        assert_eq!(toggle("  x()", 3, 3, "script").unwrap().text, "--   x()");
    }
    #[test]
    fn xml_wrap_roundtrip() {
        let s = "<a/>\n<b/>";
        let e = toggle(s, 0, s.chars().count(), "xml").unwrap();
        assert_eq!(e.text, "<!-- <a/>\n<b/> -->");
        assert_eq!(
            toggle(&e.text, e.primary, e.secondary, "xml").unwrap().text,
            s
        );
    }
}
