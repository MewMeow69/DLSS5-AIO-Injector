use std::path::Path;

pub fn read(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|t| t.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

pub fn get(lines: &[String], section: &str, key: &str) -> Option<String> {
    let header = format!("[{}]", section);
    let mut in_sec = false;
    for line in lines {
        let t = line.trim();
        if t.starts_with('[') {
            in_sec = t.eq_ignore_ascii_case(&header);
            continue;
        }
        if !in_sec || t.is_empty() || t.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            if k.trim().eq_ignore_ascii_case(key) {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

pub fn get_bool(lines: &[String], section: &str, key: &str) -> Option<bool> {
    get(lines, section, key).map(|v| {
        let l = v.to_lowercase();
        l == "true" || l == "1" || l == "yes" || l == "on"
    })
}

pub fn get_f64(lines: &[String], section: &str, key: &str) -> Option<f64> {
    get(lines, section, key).and_then(|v| v.parse::<f64>().ok())
}

pub fn get_u32(lines: &[String], section: &str, key: &str) -> Option<u32> {
    get(lines, section, key).and_then(|v| v.parse::<u32>().ok())
}

pub fn set(lines: &mut Vec<String>, section: &str, key: &str, value: &str) {
    let sec_header = format!("[{}]", section);
    let mut in_sec = false;
    let mut done = false;
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim().to_string();
        if t.starts_with('[') {
            if in_sec && !done {
                lines.insert(i, format!("{} = {}", key, value));
                done = true;
                break;
            }
            in_sec = t.eq_ignore_ascii_case(&sec_header);
        } else if in_sec && !t.starts_with(';') {
            if let Some((k, _)) = t.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    if done {
                        lines.remove(i);
                        continue;
                    }
                    lines[i] = format!("{} = {}", key, value);
                    done = true;
                }
            }
        }
        i += 1;
    }
    if !done {
        lines.push(String::new());
        lines.push(sec_header);
        lines.push(format!("{} = {}", key, value));
    }
}
