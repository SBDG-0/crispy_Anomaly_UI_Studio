use regex::{Regex, RegexBuilder};
use std::ops::Range;
#[derive(Clone, Default)]
pub struct Find {
    pub open: bool,
    pub replace_open: bool,
    pub query: String,
    pub replacement: String,
    pub case: bool,
    pub word: bool,
    pub regex: bool,
    pub preserve: bool,
    pub current: usize,
    pub focus: bool,
    pub error: String,
}
impl Find {
    pub fn expression(&self) -> Result<Option<Regex>, String> {
        if self.query.is_empty() {
            return Ok(None);
        }
        let q = if self.regex {
            self.query.clone()
        } else {
            regex::escape(&self.query)
        };
        let q = if self.word {
            format!(r"\b(?:{q})\b")
        } else {
            q
        };
        RegexBuilder::new(&q)
            .case_insensitive(!self.case)
            .multi_line(true)
            .size_limit(8 * 1024 * 1024)
            .build()
            .map(Some)
            .map_err(|e| e.to_string())
    }
    pub fn matches(&self, text: &str) -> Result<Vec<Range<usize>>, String> {
        let Some(re) = self.expression()? else {
            return Ok(vec![]);
        };
        let found: Vec<_> = re.find_iter(text).take(100001).map(|m| m.range()).collect();
        if found.len() > 100000 {
            return Err("More than 100,000 matches; refine the search.".into());
        }
        Ok(found)
    }
    pub fn replace(
        &self,
        text: &str,
        all: bool,
    ) -> Result<(String, Option<Range<usize>>, usize), String> {
        let Some(re) = self.expression()? else {
            return Ok((text.into(), None, 0));
        };
        let mut edits = vec![];
        for (i, caps) in re.captures_iter(text).take(100001).enumerate() {
            if !all && i != self.current {
                continue;
            }
            let m = caps.get(0).unwrap();
            let mut value = String::new();
            if self.regex {
                caps.expand(&self.replacement.replace("$&", "$0"), &mut value);
            } else {
                value = self.replacement.clone();
            }
            if self.preserve {
                value = preserve_case(m.as_str(), &value);
            }
            edits.push((m.range(), value));
            if !all {
                break;
            }
        }
        if edits.len() > 100000 {
            return Err("More than 100,000 replacements; refine the search.".into());
        }
        let chosen = edits.first().map(|(r, v)| r.start..r.start + v.len());
        let count = edits.len();
        let mut out: String = text.into();
        for (r, v) in edits.into_iter().rev() {
            out.replace_range(r, &v)
        }
        Ok((out, chosen, count))
    }
}
pub fn preserve_case(original: &str, replacement: &str) -> String {
    fn part(a: &str, b: &str) -> String {
        let letters: Vec<_> = a.chars().filter(|c| c.is_alphabetic()).collect();
        if letters.is_empty() {
            return b.into();
        }
        if letters.iter().all(|c| c.is_uppercase()) {
            b.to_uppercase()
        } else if letters.iter().all(|c| c.is_lowercase()) {
            b.to_lowercase()
        } else if letters[0].is_uppercase() && letters[1..].iter().all(|c| c.is_lowercase()) {
            let mut chars = b.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase())
                .unwrap_or_default()
        } else {
            b.into()
        }
    }
    for separator in ['_', '-'] {
        let a: Vec<_> = original.split(separator).collect();
        let b: Vec<_> = replacement.split(separator).collect();
        if a.len() > 1 && a.len() == b.len() {
            return a
                .iter()
                .zip(b)
                .map(|(a, b)| part(a, b))
                .collect::<Vec<_>>()
                .join(&separator.to_string());
        }
    }
    part(original, replacement)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn case_word_and_regex() {
        let mut f = Find {
            query: "cat".into(),
            word: true,
            ..Default::default()
        };
        assert_eq!(f.matches("cat CAT scatter cat_foo").unwrap().len(), 2);
        f.case = true;
        assert_eq!(f.matches("cat CAT").unwrap().len(), 1);
        f.regex = true;
        f.query = "[".into();
        assert!(f.matches("a").is_err());
    }
    #[test]
    fn capture_replacement_and_case() {
        let f = Find {
            query: r"(foo)(\d+)".into(),
            replacement: "$2-$1".into(),
            regex: true,
            ..Default::default()
        };
        assert_eq!(f.replace("foo12 foo7", true).unwrap().0, "12-foo 7-foo");
        assert_eq!(preserve_case("HELLO_world", "new_thing"), "NEW_thing");
        assert_eq!(preserve_case("Hello", "world"), "World");
    }
    #[test]
    fn zero_width_unicode_replace_and_selected_match() {
        let f = Find {
            query: r"^".into(),
            replacement: "; ".into(),
            regex: true,
            ..Default::default()
        };
        assert_eq!(f.replace("é\n中", true).unwrap().0, "; é\n; 中");
        let f = Find {
            query: "a".into(),
            replacement: "b".into(),
            current: 1,
            ..Default::default()
        };
        assert_eq!(f.replace("a a a", false).unwrap().0, "a b a");
    }
}
