use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io, path::Path};

pub(super) type Section = BTreeMap<String, String>;
pub(super) type Ini = BTreeMap<String, Section>;

pub(super) fn read_file(path: &Path, label: &str) -> Result<(Ini, String), String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok((Ini::new(), "missing".into()));
        }
        Err(_) => {
            return Err(format!(
                "{label}을(를) 읽을 수 없습니다. 경로와 권한을 확인해 주세요."
            ));
        }
    };
    let sections =
        parse_ini(&contents).map_err(|()| format!("{label}의 INI 형식을 확인해 주세요."))?;
    Ok((
        sections,
        format!("{:x}", Sha256::digest(contents.as_bytes())),
    ))
}

fn parse_ini(contents: &str) -> Result<Ini, ()> {
    let mut sections = Ini::new();
    let mut section = None;
    let mut previous_key: Option<(String, usize)> = None;
    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        let indent = raw.len() - raw.trim_start().len();
        if let (Some(name), Some((key, previous_indent))) = (section, &previous_key)
            && indent > *previous_indent
        {
            let value = sections
                .get_mut(name)
                .and_then(|fields| fields.get_mut(key))
                .ok_or(())?;
            value.push('\n');
            value.push_str(line);
            continue;
        }
        if let Some(header) = line.strip_prefix('[').and_then(|line| line.split_once(']')) {
            // Python's ConfigParser treats [DEFAULT] as inherited options. Handle it below.
            if sections.contains_key(header.0) || header.0.is_empty() {
                return Err(());
            }
            sections.insert(header.0.to_string(), Section::new());
            section = Some(header.0);
            previous_key = None;
            continue;
        }
        let fields = sections.get_mut(section.ok_or(())?).ok_or(())?;
        let separator = line.find(['=', ':']).ok_or(())?;
        let key = line[..separator].trim_end().to_ascii_lowercase();
        if key.is_empty()
            || fields
                .insert(key.clone(), line[separator + 1..].trim().into())
                .is_some()
        {
            return Err(());
        }
        previous_key = Some((key, indent));
    }
    if let Some(defaults) = sections.remove("DEFAULT") {
        for section in sections.values_mut() {
            for (key, value) in &defaults {
                section.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
    }
    Ok(sections)
}
