use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum Vdf {
    Str(String),
    Obj(BTreeMap<String, Vdf>),
}

impl Vdf {
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(m) => m.get(key),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn obj(&self) -> Option<&BTreeMap<String, Vdf>> {
        match self {
            Vdf::Obj(m) => Some(m),
            _ => None,
        }
    }
}

pub fn parse(text: &str) -> Vdf {
    let mut chars = text.chars().peekable();
    parse_obj(&mut chars, false)
}

fn skip_ws(chars: &mut std::iter::Peekable<std::str::Chars>) {
    loop {
        match chars.peek() {
            Some(c) if c.is_whitespace() => {
                chars.next();
            }
            Some('/') => {
                let mut probe = chars.clone();
                probe.next();
                if probe.peek() == Some(&'/') {
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
}

fn parse_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut out = String::new();
    skip_ws(chars);
    if chars.peek() == Some(&'"') {
        chars.next();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => {
                    if let Some(n) = chars.next() {
                        out.push(n);
                    }
                }
                _ => out.push(c),
            }
        }
    } else {
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '{' || c == '}' {
                break;
            }
            out.push(c);
            chars.next();
        }
    }
    out
}

fn parse_obj(chars: &mut std::iter::Peekable<std::str::Chars>, braced: bool) -> Vdf {
    let mut map = BTreeMap::new();
    loop {
        skip_ws(chars);
        match chars.peek() {
            None => break,
            Some('}') => {
                chars.next();
                break;
            }
            _ => {}
        }
        let key = parse_string(chars);
        if key.is_empty() {
            chars.next();
            continue;
        }
        skip_ws(chars);
        if chars.peek() == Some(&'{') {
            chars.next();
            map.insert(key, parse_obj(chars, true));
        } else {
            let val = parse_string(chars);
            map.insert(key, Vdf::Str(val));
        }
        if !braced && map.len() > 8192 {
            break;
        }
    }
    Vdf::Obj(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACF: &str = r#"
"AppState"
{
	"appid"		"244210"
	"name"		"Assetto Corsa"
	"installdir"		"assettocorsa"
	"SizeOnDisk"		"44949470831"
	"StateFlags"		"4"
	"UserConfig"
	{
		"language"		"english"
	}
}
"#;

    #[test]
    fn parses_acf() {
        let tree = parse(ACF);
        let s = tree.get("AppState").expect("AppState");
        assert_eq!(s.get("appid").and_then(|v| v.str()), Some("244210"));
        assert_eq!(s.get("installdir").and_then(|v| v.str()), Some("assettocorsa"));
        let uc = s.get("UserConfig").expect("nested");
        assert_eq!(uc.get("language").and_then(|v| v.str()), Some("english"));
    }

    #[test]
    fn parses_library_paths() {
        let text = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
	}
	"1"
	{
		"path"		"E:\\Gameria\\Steamer"
	}
	// comment
	"2"		"Q:\\SteamLibrary"
}
"#;
        let tree = parse(text);
        let root = tree.get("libraryfolders").unwrap();
        let obj = root.obj().unwrap();
        let mut paths: Vec<String> = Vec::new();
        for (_k, v) in obj {
            match v {
                Vdf::Str(s) => paths.push(s.clone()),
                Vdf::Obj(_) => paths.push(v.get("path").and_then(|p| p.str()).unwrap_or("").to_string()),
            }
        }
        assert!(paths.iter().any(|p| p.contains("Gameria")));
        assert!(paths.iter().any(|p| p == "Q:\\SteamLibrary"));
    }
}
