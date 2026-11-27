use crate::document::Document;
use std::{
    collections::BTreeMap,
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Field {
    pub value: String,
    pub span: Range<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Inventory,
    Upgrade,
    XmlRegion,
    XmlElement,
    XmlString,
}
#[derive(Clone, Debug)]
pub struct Icon {
    pub kind: IconKind,
    pub name: String,
    pub operator: String,
    pub texture: String,
    pub rect: [f32; 4],
    pub scale: f32,
    pub unit: f32,
    pub fields: BTreeMap<String, Field>,
    pub node: Option<usize>,
    pub header: usize,
}
impl Icon {
    pub fn label(&self) -> String {
        match self.kind {
            IconKind::Upgrade => format!("{} {} · Upgrade", self.operator, self.name),
            IconKind::XmlString => format!("{} · String", self.name),
            _ => format!("{} {}", self.operator, self.name),
        }
    }
    pub fn source_start(&self, source: &str) -> usize {
        if self.node.is_some() {
            self.header
        } else {
            let head = source[..self.header.min(source.len())].trim_end_matches(['\r', '\n']);
            head.rfind('\n').map(|i| i + 1).unwrap_or(0)
        }
    }
    pub fn pixels(&self) -> [f32; 4] {
        self.rect.map(|v| v * self.unit)
    }
    pub fn key(&self, i: usize) -> &str {
        if self.kind == IconKind::Upgrade {
            [
                "upgr_icon_x",
                "upgr_icon_y",
                "upgr_icon_width",
                "upgr_icon_height",
            ][i]
        } else if self.node.is_some() {
            ["x", "y", "width", "height"][i]
        } else {
            [
                "inv_grid_x",
                "inv_grid_y",
                "inv_grid_width",
                "inv_grid_height",
            ][i]
        }
    }
}
pub fn parse(source: &str, xml: bool) -> Result<Vec<Icon>, String> {
    if xml {
        let doc = Document::parse(source.into(), false, false)?;
        let mut icons = Vec::new();
        for (i, n) in doc.nodes.iter().enumerate() {
            if n.name != "texture" || n.get("id").is_empty() {
                continue;
            }
            let Some(p) = n.parent else { continue };
            let file = &doc.nodes[p];
            if file.name != "file" {
                continue;
            }
            let mut fields: BTreeMap<String, Field> = n
                .attrs
                .iter()
                .map(|(k, a)| {
                    (
                        k.clone(),
                        Field {
                            value: a.value.clone(),
                            span: a.span.clone(),
                        },
                    )
                })
                .collect();
            if let Some(a) = file.attrs.get("name") {
                fields.insert(
                    "__atlas_texture".into(),
                    Field {
                        value: a.value.clone(),
                        span: a.span.clone(),
                    },
                );
            }
            icons.push(Icon {
                kind: IconKind::XmlRegion,
                name: n.get("id").into(),
                operator: "XML".into(),
                texture: file.get("name").into(),
                rect: [n.num("x"), n.num("y"), n.num("width"), n.num("height")],
                scale: 1.0,
                unit: 1.0,
                fields,
                node: Some(i),
                header: n.span.start,
            });
        }
        for (i, n) in doc.nodes.iter().enumerate() {
            if n.drawable() {
                let texture = doc
                    .texture(i)
                    .map(|t| t.text.trim().to_owned())
                    .unwrap_or_default();
                let mut fields: BTreeMap<String, Field> = n
                    .attrs
                    .iter()
                    .map(|(k, a)| {
                        (
                            k.clone(),
                            Field {
                                value: a.value.clone(),
                                span: a.span.clone(),
                            },
                        )
                    })
                    .collect();
                if let Some(t) = doc.texture(i) {
                    if let Some(span) = &t.text_span {
                        fields.insert(
                            "__atlas_texture".into(),
                            Field {
                                value: t.text.clone(),
                                span: span.clone(),
                            },
                        );
                    }
                }
                if let Some(t) = doc
                    .nodes
                    .iter()
                    .find(|t| t.parent == Some(i) && t.name == "text")
                {
                    if let Some(span) = &t.text_span {
                        fields.insert(
                            "__text".into(),
                            Field {
                                value: t.text.clone(),
                                span: span.clone(),
                            },
                        );
                    }
                }
                icons.push(Icon {
                    kind: IconKind::XmlElement,
                    name: if !n.get("id").is_empty() {
                        n.get("id").into()
                    } else if !n.get("name").is_empty() {
                        format!("{} ({})", n.name, n.get("name"))
                    } else {
                        format!("{} #{}", n.name, i)
                    },
                    operator: "XML".into(),
                    texture,
                    rect: [n.num("x"), n.num("y"), n.num("width"), n.num("height")],
                    unit: 1.,
                    scale: 1.,
                    fields,
                    node: Some(i),
                    header: n.span.start,
                });
            }
            if n.name == "string" && !n.get("id").is_empty() {
                if let Some((j, t)) = doc
                    .nodes
                    .iter()
                    .enumerate()
                    .find(|(_, t)| t.parent == Some(i) && t.name == "text")
                {
                    let mut fields = BTreeMap::new();
                    if let Some(span) = &t.text_span {
                        fields.insert(
                            "__text".into(),
                            Field {
                                value: t.text.clone(),
                                span: span.clone(),
                            },
                        );
                    }
                    icons.push(Icon {
                        kind: IconKind::XmlString,
                        name: n.get("id").into(),
                        operator: "XML".into(),
                        texture: String::new(),
                        rect: [0., 0., 0., 0.],
                        unit: 1.,
                        scale: 1.,
                        fields,
                        node: Some(j),
                        header: n.span.start,
                    });
                }
            }
        }
        return Ok(icons);
    }
    let mut out: Vec<Icon> = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let content = line.split(';').next().unwrap_or("").trim();
        if let Some(open) = content.find('[') {
            let prefix = content[..open].trim();
            if prefix.chars().all(|c| "!@".contains(c)) {
                if let Some(close) = content[open + 1..].find(']') {
                    out.push(Icon {
                        kind: IconKind::Inventory,
                        name: content[open + 1..open + 1 + close].into(),
                        operator: prefix.into(),
                        texture: String::new(),
                        rect: [0., 0., 1., 1.],
                        scale: 1.,
                        unit: 50.,
                        fields: BTreeMap::new(),
                        node: None,
                        header: offset + line.len(),
                    });
                }
            }
        } else if let Some(icon) = out.last_mut() {
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                let tail = &line[eq + 1..];
                let value = tail.split(';').next().unwrap_or("").trim();
                let leading = tail.len() - tail.trim_start().len();
                let start = offset + eq + 1 + leading;
                icon.fields.insert(
                    key.into(),
                    Field {
                        value: value.into(),
                        span: start..start + value.len(),
                    },
                );
            }
        }
        offset += line.len();
    }
    for icon in &mut out {
        icon.texture = icon
            .fields
            .get("icons_texture")
            .or_else(|| icon.fields.get("texture"))
            .map(|f| f.value.trim_matches('"').into())
            .unwrap_or_else(|| "ui\\ui_icon_equipment".into());
        for (i, k) in [
            "inv_grid_x",
            "inv_grid_y",
            "inv_grid_width",
            "inv_grid_height",
        ]
        .iter()
        .enumerate()
        {
            if let Some(f) = icon.fields.get(*k) {
                if let Ok(v) = f.value.parse() {
                    icon.rect[i] = v;
                }
            }
        }
        icon.scale = icon
            .fields
            .get("inv_grid_scale")
            .and_then(|f| f.value.parse().ok())
            .unwrap_or(1.);
    }
    let mut expanded = Vec::new();
    for mut icon in out {
        let upgrade = icon.fields.contains_key("upgr_icon_path")
            || icon.fields.keys().any(|k| k.starts_with("upgr_icon_"));
        let mut u = icon.clone();
        if upgrade {
            icon.fields.retain(|k, _| !k.starts_with("upgr_icon_"));
        }
        expanded.push(icon);
        if upgrade {
            u.kind = IconKind::Upgrade;
            u.unit = 1.;
            u.scale = 1.;
            u.texture = u
                .fields
                .get("upgr_icon_path")
                .map(|f| f.value.trim_matches('"').into())
                .unwrap_or_default();
            for j in 0..4 {
                u.rect[j] = u
                    .fields
                    .get(u.key(j))
                    .and_then(|f| f.value.parse().ok())
                    .unwrap_or(0.);
            }
            u.fields.retain(|k, _| k.starts_with("upgr_icon_"));
            expanded.push(u);
        }
    }
    Ok(expanded)
}
pub fn change(source: &str, icon: &Icon, key: &str, value: &str) -> Result<String, String> {
    let escaped = icon
        .node
        .map(|_| crate::xml_tools::escape(value, key != "__text"));
    let value = escaped.as_deref().unwrap_or(value);
    let mut s = source.to_owned();
    if let Some(f) = icon.fields.get(key) {
        s.replace_range(f.span.clone(), value)
    } else if icon.node.is_none() {
        let nl = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let separator = if icon.header > 0 && !source[..icon.header].ends_with('\n') {
            nl
        } else {
            ""
        };
        s.insert_str(icon.header, &format!("{separator}{key} = {value}{nl}"));
    } else {
        let d = Document::parse(source.into(), false, false)?;
        let n = &d.nodes[icon.node.unwrap()];
        let end = n.open_end;
        if key == "__text" {
            if source[..end].ends_with("/>") {
                s.replace_range(end - 2..end, &format!(">{value}</{}>", n.name));
            } else {
                s.insert_str(end, value)
            }
        } else {
            let insert = if source[..end].ends_with("/>") {
                end - 2
            } else {
                end - 1
            };
            s.insert_str(insert, &format!(" {key}=\"{value}\""));
        }
    }
    Ok(s)
}
pub fn texture_path(roots: &[PathBuf], name: &str) -> Option<PathBuf> {
    let mut relative = PathBuf::from("textures").join(name.trim_matches('"').replace('\\', "/"));
    if relative.extension().is_none() {
        relative.set_extension("dds");
    }
    roots
        .iter()
        .rev()
        .map(|r| r.join(&relative))
        .find(|p| p.is_file())
}
pub fn lua(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', "\\r")
            .replace('\n', "\\n")
    )
}
pub fn duplicate(source: &str, index: usize, name: &str) -> Result<String, String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
    {
        return Err("Use a nonempty identifier containing letters, digits, _ or -.".into());
    }
    let icons = parse(source, false)?;
    if icons.iter().any(|i| i.name.eq_ignore_ascii_case(name)) {
        return Err("A section with this name already exists in this file".into());
    }
    let icon = icons.get(index).ok_or("Select a section")?;
    let head = source[..icon.header].trim_end_matches(['\r', '\n']);
    let start = head.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let end = icons
        .iter()
        .find(|next| next.header > icon.header)
        .map(|next| {
            let h = source[..next.header].trim_end_matches(['\r', '\n']);
            h.rfind('\n').map(|p| p + 1).unwrap_or(source.len())
        })
        .unwrap_or(source.len());
    let raw = &source[start..end];
    let close = raw.find(']').ok_or("Invalid section header")?;
    let nl = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    Ok(format!("{source}{nl}[{name}]{}", &raw[close + 1..]))
}
pub fn patch_ltx(icon: &Icon, override_file: bool) -> String {
    if icon.operator == "!!" {
        return format!("; Generated by Anomaly Icon Studio\n!![{}]\n", icon.name);
    }
    let mut s = format!(
        "; Generated by Anomaly Icon Studio\n; Target: {}\n@[{}]\n",
        if override_file {
            "Icon Override"
        } else {
            "system"
        },
        icon.name
    );
    // Export only explicitly defined fields. Synthesizing defaults here would
    // accidentally replace values inherited from a base section or another mod.
    for (key, field) in &icon.fields {
        if key == "icons_texture"
            || key == "texture"
            || key.starts_with("inv_grid_")
            || key.starts_with("upgr_icon_")
            || key.contains("icon_layer")
        {
            s.push_str(&format!("{key} = {}\n", field.value));
        }
    }
    s
}
fn selector(doc: &Document, index: usize) -> Result<String, String> {
    let node = &doc.nodes[index];
    for key in ["id", "name"] {
        let value = node.get(key);
        if !value.is_empty()
            && !value
                .chars()
                .any(|c| c.is_whitespace() || "[]\"'".contains(c))
            && doc
                .nodes
                .iter()
                .filter(|n| n.name == node.name && n.get(key) == value)
                .count()
                == 1
        {
            return Ok(format!("{}[{key}={value}]", node.name));
        }
    }
    if doc.nodes.iter().filter(|n| n.name == node.name).count() == 1 {
        return Ok(node.name.clone());
    }
    Err(format!(
        "Ambiguous DXML target <{}>; use a unique id or name",
        node.name
    ))
}
pub fn dxml(original: &str, current: &str, relative: &Path) -> Result<String, String> {
    let before = Document::parse(original.into(), false, false)?;
    let after = Document::parse(current.into(), false, false)?;
    if before.nodes.len() != after.nodes.len() {
        return Err(
            "DXML export currently supports attribute changes, not adding/removing elements".into(),
        );
    }
    let mut body = String::new();
    for (i, n) in after.nodes.iter().enumerate() {
        let old = &before.nodes[i];
        if old.name != n.name || old.text != n.text || old.attrs.keys().ne(n.attrs.keys()) {
            return Err("DXML export supports existing attributes only; use direct XML export for structural/text edits".into());
        }
        let changes: Vec<_> = n
            .attrs
            .iter()
            .filter(|(k, a)| old.get(k) != a.value)
            .collect();
        if changes.is_empty() {
            continue;
        }
        let selector = selector(&before, i)?;
        body.push_str(&format!("        local matches = xml_obj:query({})\n        if #matches == 1 then\n            xml_obj:setElementAttr(matches[1], {{",lua(&selector)));
        for (key, a) in changes {
            body.push_str(&format!("[{}] = {},", lua(key), lua(&a.value)));
        }
        body.push_str("})\n        end\n");
    }
    if body.is_empty() {
        return Err("No attribute changes to export".into());
    }
    let path = relative.to_string_lossy().replace('/', "\\");
    Ok(format!("-- Generated by Anomaly Icon Studio; requires DXML\nfunction on_xml_read()\n    RegisterScriptCallback(\"on_xml_read\", function(xml_file_name, xml_obj)\n        if xml_file_name ~= {} then return end\n{}    end)\nend\n",lua(&path),body))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrade_coordinates_are_pixels_and_do_not_overwrite_inventory_fields() {
        let s="![suit]\nicons_texture=ui/base\ninv_grid_x=3\ninv_grid_y=4\ninv_grid_width=2\ninv_grid_height=1\nupgr_icon_path=ui/armor\nupgr_icon_x=600\nupgr_icon_y=0\nupgr_icon_width=300\nupgr_icon_height=150\n";
        let icons = parse(s, false).unwrap();
        assert_eq!(icons.len(), 2);
        let upgrade = &icons[1];
        assert_eq!(upgrade.kind, IconKind::Upgrade);
        assert_eq!(upgrade.pixels(), [600., 0., 300., 150.]);
        let edited = change(s, upgrade, upgrade.key(0), "650").unwrap();
        assert!(edited.contains("inv_grid_x=3"));
        assert!(edited.contains("upgr_icon_x=650"));
        assert!(!patch_ltx(upgrade, false).contains("inv_grid_x"));
    }
    #[test]
    fn xml_strings_and_positioned_elements_roundtrip() {
        let s="<w><string id='hello'><text>Hi &amp; bye</text></string><panel width='100' height='50'><texture>ui/bg</texture><text>caption</text></panel></w>";
        let icons = parse(s, true).unwrap();
        let string = icons
            .iter()
            .find(|i| i.kind == IconKind::XmlString)
            .unwrap();
        let edited = change(s, string, "__text", "Rock & roll <3").unwrap();
        assert!(edited.contains("Rock &amp; roll &lt;3"));
        let panel = icons
            .iter()
            .find(|i| i.kind == IconKind::XmlElement)
            .unwrap();
        let moved = change(s, panel, "x", "42").unwrap();
        let result = parse(&moved, true).unwrap();
        assert_eq!(
            result
                .iter()
                .find(|i| i.kind == IconKind::XmlElement)
                .unwrap()
                .rect[0],
            42.
        );
        assert!(moved.contains("<text>caption</text>"));
    }
    #[test]
    fn edits_preserve_crlf_comments_and_repeated_sections() {
        let s = "; hello\r\n@[axe]\r\ninv_grid_x = 3 ; keep\r\n@[axe]\r\ninv_grid_x=8\r\n";
        let icons = parse(s, false).unwrap();
        assert_eq!(icons.len(), 2);
        assert_eq!(
            change(s, &icons[0], "inv_grid_x", "9").unwrap(),
            s.replacen("3 ;", "9 ;", 1)
        );
        assert_eq!(icons[1].rect[0], 8.);
    }
    #[test]
    fn patch_uses_real_override_section() {
        let i = parse("@[a]\ninv_grid_scale=2\ninv_grid_width=4\n", false)
            .unwrap()
            .remove(0);
        assert_eq!(i.pixels()[2], 200.);
        assert!(patch_ltx(&i, true).contains("@[a]"));
    }
    #[test]
    fn xml_geometry_and_dxml() {
        let a="<w><file name='ui/a'><texture id='one' x='0' y='0' width='50' height='50'/></file></w>";
        let i = parse(a, true).unwrap().remove(0);
        let b = change(a, &i, "x", "50").unwrap();
        let script = dxml(a, &b, Path::new("ui/textures_descr/a.xml")).unwrap();
        assert!(script.contains("texture[id=one]"));
        assert!(script.contains("[\"x\"] = \"50\""));
        assert!(script.contains("ui\\\\textures_descr\\\\a.xml"));
    }
    #[test]
    fn dxml_rejects_ambiguous_targets() {
        let a = "<w><a id='x' x='1'/><a id='x' x='1'/></w>";
        assert!(dxml(a, &a.replacen("x='1'", "x='2'", 1), Path::new("a.xml")).is_err());
    }
    #[test]
    fn operators_preserved() {
        for op in ["", "!", "!!", "@"] {
            assert_eq!(
                parse(&format!("{op}[test]\nx=1"), false).unwrap()[0].operator,
                op
            );
        }
    }
    #[test]
    fn duplicate_keeps_inheritance_and_non_icon_fields() {
        let source = "[a]:base\r\ncost=100 ; keep\r\ninv_grid_x=2\r\n[b]\r\ncost=200\r\n";
        let text = duplicate(source, 0, "new_item").unwrap();
        assert!(text.ends_with("[new_item]:base\r\ncost=100 ; keep\r\ninv_grid_x=2\r\n"));
        assert!(duplicate(source, 0, "a").is_err());
    }
    #[test]
    fn absent_fields_are_not_synthesized_in_patch() {
        let i = parse("![a]\ninv_grid_x=3\n", false).unwrap().remove(0);
        let patch = patch_ltx(&i, false);
        assert!(patch.contains("inv_grid_x = 3"));
        assert!(!patch.contains("inv_grid_scale"));
        assert!(!patch.contains("icons_texture"));
    }
    #[test]
    fn xml_parent_texture_updates_all_regions() {
        let source="<w><file name='ui/a'><texture id='a' x='0' y='0' width='50' height='50'/><texture id='b' x='50' y='0' width='50' height='50'/></file></w>";
        let icons = parse(source, true).unwrap();
        let output = change(source, &icons[0], "__atlas_texture", "ui/b").unwrap();
        let updated = parse(&output, true).unwrap();
        assert!(updated.iter().all(|i| i.texture == "ui/b"));
        assert!(dxml(source, &output, Path::new("ui/a.xml"))
            .unwrap()
            .contains("file[name=ui/a]"));
    }
}
