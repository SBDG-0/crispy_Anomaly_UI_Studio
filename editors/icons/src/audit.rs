use std::path::Path;
pub fn run(root: &Path) -> bool {
    let mut counts = [0usize; 4];
    let mut errors = vec![];
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "dds" => {
                counts[0] += 1;
                if let Err(e) = crate::assets::decode(path) {
                    errors.push(format!("DDS {}: {e}", path.display()));
                }
            }
            "ltx" | "xml" => {
                let xml = path
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("xml"));
                counts[if xml { 2 } else { 1 }] += 1;
                let bytes = std::fs::read(path).unwrap_or_default();
                let bytes = bytes.strip_prefix(&[239, 187, 191]).unwrap_or(&bytes);
                let s = std::str::from_utf8(bytes)
                    .map(str::to_owned)
                    .unwrap_or_else(|_| encoding_rs::WINDOWS_1251.decode(bytes).0.into_owned());
                match crate::model::parse(&s, xml) {
                    Ok(icons) => counts[3] += icons.len(),
                    Err(e) => errors.push(format!("CONFIG {}: {e}", path.display())),
                }
            }
            _ => (),
        }
    }
    println!(
        "DDS: {}\nLTX: {}\nXML: {}\nSections/regions: {}\nErrors: {}",
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        errors.len()
    );
    for e in &errors {
        println!("{e}")
    }
    errors.is_empty()
}
