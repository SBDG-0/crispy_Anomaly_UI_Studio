#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}
impl Selection {
    pub fn caret(n: usize) -> Self {
        Self { anchor: n, head: n }
    }
    pub fn range(self) -> std::ops::Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
}
pub fn byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map(|x| x.0)
        .unwrap_or(text.len())
}
pub fn line_col(text: &str, index: usize) -> (usize, usize) {
    let before = &text[..byte(text, index)];
    (
        before.matches('\n').count(),
        before
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| *c != '\r')
            .count(),
    )
}
pub fn position(text: &str, line: usize, column: usize) -> usize {
    let mut start = 0;
    for (i, s) in text.split('\n').enumerate() {
        if i == line {
            return start + column.min(s.trim_end_matches('\r').chars().count());
        }
        start += s.chars().count() + 1;
    }
    text.chars().count()
}
pub fn word_range(text: &str, index: usize) -> std::ops::Range<usize> {
    let c: Vec<_> = text.chars().collect();
    let mut a = index.min(c.len());
    let mut b = a;
    while a > 0 && word(c[a - 1]) {
        a -= 1
    }
    while b < c.len() && word(c[b]) {
        b += 1
    }
    a..b
}
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
pub fn word_move(text: &str, index: usize, right: bool) -> usize {
    let c: Vec<_> = text.chars().collect();
    let mut i = index.min(c.len());
    if right {
        if i < c.len() {
            let group = word(c[i]);
            while i < c.len() && word(c[i]) == group && !c[i].is_whitespace() {
                i += 1
            }
        }
        while i < c.len() && c[i].is_whitespace() {
            i += 1
        }
    } else {
        while i > 0 && c[i - 1].is_whitespace() {
            i -= 1
        }
        if i > 0 {
            let group = word(c[i - 1]);
            while i > 0 && word(c[i - 1]) == group && !c[i - 1].is_whitespace() {
                i -= 1
            }
        }
    }
    i
}
pub fn normalize(mut ranges: Vec<Selection>) -> Vec<Selection> {
    ranges.sort_by_key(|s| (s.range().start, s.range().end));
    let mut out: Vec<Selection> = vec![];
    for s in ranges {
        if let Some(last) = out.last_mut() {
            if s.range().start <= last.range().end
                && !(s.range().start == last.range().end && s.range().start != s.range().end)
            {
                last.anchor = last.range().start;
                last.head = last.range().end.max(s.range().end);
                continue;
            }
        }
        out.push(s)
    }
    out
}
pub fn insert(text: &str, ranges: &[Selection], values: &[String]) -> (String, Vec<Selection>) {
    let ranges = normalize(ranges.to_vec());
    let mut out = text.to_owned();
    let mut delta = 0isize;
    let mut result = vec![];
    let mut edits = vec![];
    for (i, s) in ranges.iter().enumerate() {
        let r = s.range();
        let value = values
            .get(i)
            .or_else(|| values.first())
            .cloned()
            .unwrap_or_default();
        let new = (r.start as isize + delta + value.chars().count() as isize) as usize;
        result.push(Selection::caret(new));
        delta += value.chars().count() as isize - (r.end - r.start) as isize;
        edits.push((byte(text, r.start)..byte(text, r.end), value));
    }
    for (r, v) in edits.into_iter().rev() {
        out.replace_range(r, &v)
    }
    (out, result)
}
pub fn delete(
    text: &str,
    ranges: &[Selection],
    forward: bool,
    words: bool,
) -> (String, Vec<Selection>) {
    let n = text.chars().count();
    let ranges: Vec<_> = ranges
        .iter()
        .map(|s| {
            let mut r = s.range();
            if r.is_empty() {
                if forward {
                    r.end = if words {
                        word_move(text, r.end, true)
                    } else {
                        (r.end + 1).min(n)
                    }
                } else {
                    r.start = if words {
                        word_move(text, r.start, false)
                    } else {
                        r.start.saturating_sub(1)
                    }
                }
            }
            Selection {
                anchor: r.start,
                head: r.end,
            }
        })
        .collect();
    insert(text, &ranges, &[String::new()])
}
pub fn line_operation(
    text: &str,
    s: Selection,
    down: bool,
    duplicate: bool,
) -> (String, Selection) {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split(nl).map(str::to_owned).collect();
    let (a, ca) = line_col(text, s.anchor);
    let (b, cb) = line_col(text, s.head);
    let lo = a.min(b);
    let mut hi = a.max(b);
    if hi > lo && ((b > a && cb == 0) || (a > b && ca == 0)) {
        hi -= 1
    }
    if hi >= lines.len() {
        return (text.into(), s);
    }
    let block: Vec<_> = lines[lo..=hi].to_vec();
    let change: isize;
    if duplicate {
        let at = if down { hi + 1 } else { lo };
        lines.splice(at..at, block);
        change = if down { (hi - lo + 1) as isize } else { 0 };
    } else if down {
        if hi + 1 >= lines.len() {
            return (text.into(), s);
        }
        let next = lines.remove(hi + 1);
        lines.insert(lo, next);
        change = 1;
    } else {
        if lo == 0 {
            return (text.into(), s);
        }
        let prev = lines.remove(lo - 1);
        lines.insert(hi, prev);
        change = -1;
    }
    let out = lines.join(nl);
    let shift = |v: usize| ((v as isize + change).max(0)) as usize;
    let selection = Selection {
        anchor: position(&out, shift(a), ca),
        head: position(&out, shift(b), cb),
    };
    (out, selection)
}
pub fn selected_lines(text: &str, ranges: &[Selection]) -> std::collections::BTreeSet<usize> {
    let mut lines = std::collections::BTreeSet::new();
    for s in ranges {
        let r = s.range();
        let (a, _) = line_col(text, r.start);
        let (mut b, col) = line_col(text, r.end);
        if b > a && col == 0 {
            b -= 1
        }
        lines.extend(a..=b);
    }
    lines
}
pub fn indent(text: &str, ranges: &[Selection], outdent: bool) -> (String, Vec<Selection>) {
    let mut edits: Vec<(std::ops::Range<usize>, String)> = vec![];
    for line in selected_lines(text, ranges) {
        let start = position(text, line, 0);
        let content = &text[byte(text, start)..];
        let count = if outdent {
            if content.starts_with('\t') {
                1
            } else {
                content.chars().take_while(|c| *c == ' ').take(4).count()
            }
        } else {
            0
        };
        edits.push((
            start..start + count,
            if outdent {
                String::new()
            } else {
                "    ".into()
            },
        ));
    }
    let map = |index: usize| {
        let mut delta = 0isize;
        for (r, s) in &edits {
            if index < r.start {
                break;
            }
            if index <= r.end {
                return (r.start as isize + delta + s.chars().count() as isize) as usize;
            }
            delta += s.chars().count() as isize - (r.end - r.start) as isize;
        }
        (index as isize + delta) as usize
    };
    let result = ranges
        .iter()
        .map(|s| Selection {
            anchor: map(s.anchor),
            head: map(s.head),
        })
        .collect();
    let mut out = text.to_owned();
    for (r, s) in edits.into_iter().rev() {
        out.replace_range(byte(text, r.start)..byte(text, r.end), &s)
    }
    (out, result)
}
pub fn comment(
    text: &str,
    ranges: &[Selection],
    extension: &str,
) -> Result<(String, Vec<Selection>), String> {
    let lines = selected_lines(text, ranges);
    let token = if extension.eq_ignore_ascii_case("script") {
        "--"
    } else {
        ";"
    };
    let relevant: Vec<_> = lines
        .iter()
        .filter_map(|n| text.lines().nth(*n))
        .filter(|s| !s.trim().is_empty())
        .collect();
    let remove = !relevant.is_empty() && relevant.iter().all(|s| s.trim_start().starts_with(token));
    let mut out = text.to_owned();
    for line in lines.iter().rev() {
        let at = position(&out, *line, 0);
        out = crate::comments::toggle_mode(
            &out,
            at,
            at,
            extension,
            if extension.eq_ignore_ascii_case("xml") {
                None
            } else {
                Some(remove)
            },
        )?
        .text;
    }
    let map = |pos: usize| {
        let (line, col) = line_col(text, pos);
        let old = text.lines().nth(line).unwrap_or("");
        let new = out.lines().nth(line).unwrap_or("");
        let prefix = old
            .chars()
            .zip(new.chars())
            .take_while(|(a, b)| a == b)
            .count();
        let delta = new.chars().count() as isize - old.chars().count() as isize;
        position(
            &out,
            line,
            if col < prefix {
                col
            } else {
                (col as isize + delta).max(prefix as isize) as usize
            },
        )
    };
    let selections = ranges
        .iter()
        .map(|s| Selection {
            anchor: map(s.anchor),
            head: map(s.head),
        })
        .collect();
    Ok((out, selections))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indent_and_comment_disjoint_lines_preserve_middle_line() {
        let text = "one\nuntouched\nthree";
        let ranges = [Selection::caret(1), Selection::caret(15)];
        let (out, ranges) = indent(text, &ranges, false);
        assert_eq!(out, "    one\nuntouched\n    three");
        assert_eq!(indent(&out, &ranges, true).0, text);
        let (out, ranges) =
            comment(text, &[Selection::caret(1), Selection::caret(15)], "ltx").unwrap();
        assert_eq!(out, "; one\nuntouched\n; three");
        assert_eq!(comment(&out, &ranges, "ltx").unwrap().0, text);
    }
    #[test]
    fn multicursor_unicode_and_overlaps() {
        let (text, s) = insert(
            "aé\n中b",
            &[Selection::caret(1), Selection::caret(4)],
            &["X".into()],
        );
        assert_eq!(text, "aXé\n中Xb");
        assert_eq!(s, vec![Selection::caret(2), Selection::caret(6)]);
        assert_eq!(delete(&text, &s, false, false).0, "aé\n中b");
    }
    #[test]
    fn move_and_duplicate_preserve_lines() {
        assert_eq!(
            line_operation("a\r\nb\r\nc", Selection::caret(3), false, false).0,
            "b\r\na\r\nc"
        );
        assert_eq!(
            line_operation("a\nb", Selection::caret(0), true, true).0,
            "a\na\nb"
        );
    }
    #[test]
    fn word_and_column() {
        assert_eq!(word_range("one two", 5), 4..7);
        assert_eq!(position("éé\nx", 1, 4), 4);
        assert_eq!(word_move("one two", 0, true), 4);
    }
}
