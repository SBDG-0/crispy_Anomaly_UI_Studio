//! Configuration inspection, based on the documented Monolith merge rules.
//! Deliberately scoped to an explicitly chosen LTX entry point, never a merge
//! of unrelated LTX files that happen to share a workspace.
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Value {
    pub text: String,
    pub file: PathBuf,
    pub line: usize,
    pub depth: i32,
    pub order: usize,
}
#[derive(Default, Clone)]
pub struct Resolved {
    pub sections: BTreeMap<String, BTreeMap<String, Value>>,
    pub warnings: Vec<String>,
    pub files: usize,
}
#[derive(Default)]
struct Section {
    exists: bool,
    base: BTreeMap<String, Value>,
    over: BTreeMap<String, Value>,
    parents: Vec<String>,
    over_parents: Vec<String>,
    lists: Vec<(char, String, Value)>,
}
struct Loader {
    vfs: BTreeMap<String, PathBuf>,
    buffers: BTreeMap<PathBuf, String>,
    sections: BTreeMap<String, Section>,
    deleted: BTreeSet<String>,
    warnings: Vec<String>,
    stack: Vec<String>,
    order: usize,
    files: usize,
}
fn norm(p: &str) -> String {
    let mut parts = Vec::new();
    for part in p.replace('\\', "/").split('/') {
        match part {
            "" | "." => (),
            ".." => {
                parts.pop();
            }
            _ => parts.push(part.to_ascii_lowercase()),
        }
    }
    parts.join("/")
}
fn comment(s: &str) -> &str {
    let mut quote = false;
    for (i, c) in s.char_indices() {
        if c == '"' {
            quote = !quote
        }
        if !quote && (c == ';' || s[i..].starts_with("//")) {
            return &s[..i];
        }
    }
    s
}
fn parents(target: &mut Vec<String>, incoming: impl Iterator<Item = String>, keep_remove: bool) {
    for p in incoming {
        let opposite = if p.starts_with('!') {
            p[1..].to_owned()
        } else {
            format!("!{p}")
        };
        target.retain(|x| *x != opposite);
        if keep_remove || !p.starts_with('!') {
            target.push(p)
        }
    }
}
fn put(map: &mut BTreeMap<String, Value>, key: String, value: Value) {
    if map.get(&key).is_none_or(|old| {
        value.depth < old.depth || (value.depth == old.depth && value.order > old.order)
    }) {
        map.insert(key, value);
    }
}
impl Loader {
    fn load(&mut self, path: &str, depth: i32) {
        let path = norm(path);
        if self.stack.contains(&path) {
            self.warnings.push(format!(
                "Include cycle: {} → {path}",
                self.stack.join(" → ")
            ));
            return;
        }
        if self.stack.len() > 128 {
            self.warnings.push("Include nesting exceeds 128".into());
            return;
        }
        let Some(physical) = self.vfs.get(&path).cloned() else {
            self.warnings.push(format!("Missing include: {path}"));
            return;
        };
        let source = if let Some(s) = self.buffers.get(&physical) {
            s.clone()
        } else {
            let bytes = match std::fs::read(&physical) {
                Ok(b) => b,
                Err(e) => {
                    self.warnings.push(format!("{}: {e}", physical.display()));
                    return;
                }
            };
            let b = bytes.strip_prefix(&[239, 187, 191]).unwrap_or(&bytes);
            std::str::from_utf8(b)
                .map(str::to_owned)
                .unwrap_or_else(|_| encoding_rs::WINDOWS_1251.decode(b).0.into_owned())
        };
        self.files += 1;
        self.stack.push(path.clone());
        let dir = path.rsplit_once('/').map(|x| x.0).unwrap_or("");
        let mut section = None;
        let mut over = false;
        for (line_no, line) in source.lines().enumerate() {
            let line = comment(line).trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with("#include") {
                if let Some(inc) = line.split('"').nth(1) {
                    let relative = norm(&format!("{dir}/{inc}"));
                    if relative.contains('*') || relative.contains('?') {
                        let matches: Vec<_> = self
                            .vfs
                            .keys()
                            .filter(|k| glob(&relative, k))
                            .cloned()
                            .collect();
                        if matches.is_empty() {
                            self.warnings.push(format!("Unmatched include {relative}"))
                        }
                        for file in matches {
                            self.load(&file, depth + 1)
                        }
                    } else {
                        self.load(&relative, depth + 1)
                    }
                }
                continue;
            }
            if let Some(open) = line.find('[') {
                let op = &line[..open];
                if ["", "!", "@", "!!"].contains(&op) {
                    if let Some(close) = line[open + 1..].find(']') {
                        let close = open + 1 + close;
                        let name = line[open + 1..close].to_lowercase();
                        if op == "!!" {
                            self.deleted.insert(name);
                            section = None;
                            continue;
                        }
                        over = op != "";
                        let sec = self.sections.entry(name.clone()).or_default();
                        if !over {
                            if sec.exists {
                                self.warnings.push(format!(
                                    "Duplicate base [{name}] at {}:{}",
                                    physical.display(),
                                    line_no + 1
                                ));
                            }
                            sec.exists = true;
                        }
                        if op == "@" {
                            sec.exists = true;
                        }
                        let incoming = line[close + 1..]
                            .strip_prefix(':')
                            .unwrap_or("")
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_lowercase);
                        if over {
                            parents(&mut sec.over_parents, incoming, true)
                        } else {
                            parents(&mut sec.parents, incoming, true)
                        }
                        section = Some(name);
                        continue;
                    }
                }
            }
            let Some(name) = &section else { continue };
            let (key, value) = line
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .unwrap_or((line, ""));
            let (op, key) = if key.starts_with(['!', '>', '<']) {
                (key.chars().next().unwrap(), &key[1..])
            } else {
                ('=', key)
            };
            if key.is_empty() {
                continue;
            }
            self.order += 1;
            let value = Value {
                text: if op == '!' {
                    "\0DELETE".into()
                } else {
                    value.into()
                },
                file: physical.clone(),
                line: line_no + 1,
                depth,
                order: self.order,
            };
            let sec = self.sections.get_mut(name).unwrap();
            if op == '>' || op == '<' {
                sec.lists.push((op, key.into(), value));
            } else if over {
                put(&mut sec.over, key.into(), value)
            } else {
                put(&mut sec.base, key.into(), value)
            }
        }
        self.stack.pop();
    }
    fn entry(&mut self, path: &str) {
        let path = norm(path);
        self.load(&path, 0);
        let parent = Path::new(&path).parent().unwrap_or(Path::new(""));
        let dir = parent.to_string_lossy();
        let stem = Path::new(&path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let prefix = format!("mod_{stem}_");
        let other_bases: Vec<_> = self
            .vfs
            .keys()
            .filter(|k| {
                Path::new(k).parent() == Some(parent)
                    && Path::new(k)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with(&format!("{stem}_"))
            })
            .filter_map(|k| {
                Path::new(k)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .collect();
        let mods: Vec<_> = self
            .vfs
            .keys()
            .filter(|k| {
                Path::new(k).parent() == Some(parent)
                    && Path::new(k)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with(&prefix)
            })
            .filter(|k| {
                !other_bases.iter().any(|b| {
                    Path::new(k)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with(&format!("mod_{b}_"))
                })
            })
            .cloned()
            .collect();
        for (i, p) in mods.iter().enumerate() {
            self.load(p, -200 * (i as i32 + 1));
        }
        let _ = dir;
    }
    fn evaluate(
        &mut self,
        name: &str,
        stack: &mut Vec<String>,
        cache: &mut BTreeMap<String, BTreeMap<String, Value>>,
    ) -> BTreeMap<String, Value> {
        if let Some(v) = cache.get(name) {
            return v.clone();
        }
        if stack.iter().any(|s| s == name) {
            self.warnings
                .push(format!("Inheritance cycle: {} → {name}", stack.join(" → ")));
            return BTreeMap::new();
        }
        let Some(sec) = self.sections.get(name) else {
            self.warnings.push(format!("Missing parent [{name}]"));
            return BTreeMap::new();
        };
        let base = sec.base.clone();
        let over = sec.over.clone();
        let mut p = sec.parents.clone();
        parents(&mut p, sec.over_parents.clone().into_iter(), false);
        let mut lists = sec.lists.clone();
        stack.push(name.into());
        let mut values = BTreeMap::new();
        for parent in p {
            if parent.starts_with('!') {
                continue;
            }
            values.extend(self.evaluate(&parent, stack, cache));
        }
        let mut local = base;
        let mut deleted_keys = BTreeSet::new();
        for (k, v) in over {
            if v.text == "\0DELETE" {
                local.remove(&k);
                deleted_keys.insert(k);
            } else {
                local.insert(k, v);
            }
        }
        for (k, v) in local {
            if v.text != "\0DELETE" {
                values.insert(k, v);
            }
        }
        // The engine orders CSV modifications by key and original insertion order.
        lists.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.order.cmp(&b.2.order)));
        for (op, key, mut v) in lists {
            if v.text.is_empty() || deleted_keys.contains(&key) {
                continue;
            }
            let old_text = values
                .get(&key)
                .map(|old| old.text.clone())
                .unwrap_or_default();
            {
                let mut parts: Vec<String> = old_text
                    .split(',')
                    .map(str::trim)
                    .filter(|x| !x.is_empty())
                    .map(str::to_owned)
                    .collect();
                let edits: Vec<_> = v
                    .text
                    .split(',')
                    .map(str::trim)
                    .filter(|x| !x.is_empty())
                    .collect();
                if op == '>' {
                    for x in edits {
                        parts.push(x.into());
                    }
                } else {
                    parts.retain(|x| !edits.contains(&x.as_str()));
                }
                v.text = parts.join(",");
                values.insert(key, v);
            }
        }
        stack.pop();
        cache.insert(name.into(), values.clone());
        values
    }
}
fn glob(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let (mut i, mut j, mut star, mut mark) = (0, 0, None, 0);
    while j < t.len() {
        if i < p.len() && (p[i] == b'?' || p[i] == t[j]) {
            i += 1;
            j += 1
        } else if i < p.len() && p[i] == b'*' {
            star = Some(i);
            i += 1;
            mark = j
        } else if let Some(s) = star {
            mark += 1;
            j = mark;
            i = s + 1
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == b'*' {
        i += 1
    }
    i == p.len()
}
pub fn resolve(roots: &[PathBuf], entry: &Path, buffers: BTreeMap<PathBuf, String>) -> Resolved {
    let mut loader = Loader {
        vfs: BTreeMap::new(),
        buffers,
        sections: BTreeMap::new(),
        deleted: BTreeSet::new(),
        warnings: vec![],
        stack: vec![],
        order: 0,
        files: 0,
    };
    for root in roots {
        for e in walkdir::WalkDir::new(root.join("configs"))
            .into_iter()
            .filter_map(Result::ok)
        {
            if e.file_type().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("ltx"))
            {
                if let Ok(relative) = e.path().strip_prefix(root) {
                    loader
                        .vfs
                        .insert(norm(&relative.to_string_lossy()), e.path().into());
                }
            }
        }
    }
    let relative = roots
        .iter()
        .find_map(|r| entry.strip_prefix(r).ok())
        .map(|p| norm(&p.to_string_lossy()))
        .unwrap_or_else(|| norm(&entry.to_string_lossy()));
    let mut root = relative.clone();
    let file = Path::new(&relative)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    if file.starts_with("mod_") {
        let dir = Path::new(&relative).parent().unwrap_or(Path::new(""));
        let found = loader
            .vfs
            .keys()
            .filter(|k| Path::new(k).parent() == Some(dir))
            .filter(|k| {
                !Path::new(k)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("mod_")
            })
            .filter(|k| {
                file.starts_with(&format!(
                    "mod_{}_",
                    Path::new(k)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                ))
            })
            .max_by_key(|k| k.len())
            .cloned();
        if let Some(p) = found {
            root = p
        } else {
            loader.warnings.push("No base LTX entry point found. Add the base game's configs as a reference; this snapshot is incomplete.".into());
        }
    }
    if !loader.vfs.contains_key(&root) {
        loader.vfs.insert(root.clone(), entry.into());
    }
    loader.entry(&root);
    let names: Vec<_> = loader.sections.keys().cloned().collect();
    let mut cache = BTreeMap::new();
    for name in names {
        if loader.deleted.contains(&name) {
            continue;
        }
        if !loader.sections[&name].exists {
            loader
                .warnings
                .push(format!("Override [{name}] has no loaded base section"));
            continue;
        }
        loader.evaluate(&name, &mut vec![], &mut cache);
    }
    for name in &loader.deleted {
        cache.remove(name);
    }
    Resolved {
        sections: cache,
        warnings: loader.warnings,
        files: loader.files,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("icon-resolve-{}-{name}", std::process::id()));
        std::fs::create_dir_all(p.join("configs")).unwrap();
        p
    }
    #[test]
    fn inheritance_depth_and_deletion() {
        let r = fixture("depth");
        std::fs::write(
            r.join("configs/system.ltx"),
            "[parent]\nx=1\nlist=a,b\n[child]:parent\nx=2\n",
        )
        .unwrap();
        std::fs::write(
            r.join("configs/mod_system_a.ltx"),
            "![parent]\nx=8\n![child]\n!x\n>list=c\n<list=a\n@[new]\ny=3\n!![parent]\n",
        )
        .unwrap();
        let v = resolve(&[r.clone()], &r.join("configs/system.ltx"), BTreeMap::new());
        assert_eq!(v.sections["child"]["x"].text, "8");
        assert_eq!(v.sections["child"]["list"].text, "b,c");
        assert_eq!(v.sections["new"]["y"].text, "3");
        assert!(!v.sections.contains_key("parent"));
    }
    #[test]
    fn later_mod_and_direct_depth_win() {
        let r = fixture("order");
        for (p, s) in [
            ("system.ltx", "[a]\nx=0\n"),
            ("inc.ltx", "![a]\nx=2\n"),
            ("mod_system_a.ltx", "![a]\nx=1\n"),
            ("mod_system_z.ltx", "#include \"inc.ltx\"\n![a]\nx=3\n"),
        ] {
            std::fs::write(r.join("configs").join(p), s).unwrap();
        }
        let v = resolve(&[r.clone()], &r.join("configs/system.ltx"), BTreeMap::new());
        assert_eq!(v.sections["a"]["x"].text, "3");
    }
    #[test]
    fn cycle_reported() {
        let r = fixture("cycle");
        std::fs::write(r.join("configs/system.ltx"), "[a]:b\n[b]:a\n").unwrap();
        let v = resolve(&[r.clone()], &r.join("configs/system.ltx"), BTreeMap::new());
        assert!(v.warnings.iter().any(|w| w.contains("cycle")));
    }
    #[test]
    fn list_add_can_create_key_and_preserve_duplicates() {
        let r = fixture("lists");
        std::fs::write(r.join("configs/system.ltx"), "[a]\nlist=x\n").unwrap();
        std::fs::write(
            r.join("configs/mod_system_a.ltx"),
            "![a]\n>list=x,y\n>new=a,b\n!blocked\n>blocked=c\n",
        )
        .unwrap();
        let v = resolve(&[r.clone()], &r.join("configs/system.ltx"), BTreeMap::new());
        assert_eq!(v.sections["a"]["list"].text, "x,x,y");
        assert_eq!(v.sections["a"]["new"].text, "a,b");
        assert!(!v.sections["a"].contains_key("blocked"));
    }
    #[test]
    fn matching_longer_basename_owns_patch() {
        let r = fixture("ambiguous");
        for (p, s) in [
            ("system.ltx", "[a]\nx=0\n"),
            ("system_extra.ltx", "[b]\nx=0\n"),
            ("mod_system_extra_a.ltx", "@[b]\nx=9\n"),
        ] {
            std::fs::write(r.join("configs").join(p), s).unwrap();
        }
        let v = resolve(&[r.clone()], &r.join("configs/system.ltx"), BTreeMap::new());
        assert!(!v.sections.contains_key("b"));
        let v = resolve(
            &[r.clone()],
            &r.join("configs/system_extra.ltx"),
            BTreeMap::new(),
        );
        assert_eq!(v.sections["b"]["x"].text, "9");
    }
    #[test]
    fn source_layer_replaces_whole_file() {
        let low = fixture("vfs-low");
        let high = fixture("vfs-high");
        std::fs::write(low.join("configs/system.ltx"), "[removed]\nx=1\n").unwrap();
        std::fs::write(high.join("configs/system.ltx"), "[new]\nx=2\n").unwrap();
        let v = resolve(
            &[low.clone(), high],
            &low.join("configs/system.ltx"),
            BTreeMap::new(),
        );
        assert!(!v.sections.contains_key("removed"));
        assert_eq!(v.sections["new"]["x"].text, "2");
    }
}
