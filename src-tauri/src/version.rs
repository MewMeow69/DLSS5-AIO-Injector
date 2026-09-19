use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub nums: Vec<u64>,
    pub suffix: String,
}

pub fn is_prerelease_text(s: &str) -> bool {
    let tokens: Vec<String> = s
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "alpha" | "beta" | "pre" | "preview" | "rc" | "dev" | "nightly" | "canary" | "snapshot"
        ) || (t.starts_with("rc") && t[2..].chars().all(|c| c.is_ascii_digit()) && t.len() > 2)
    })
}

pub fn extract(text: &str) -> Version {
    let s = text.trim().trim_start_matches(['v', 'V']);
    let (head, suffix) = match s.find('-') {
        Some(i) => (&s[..i], s[i + 1..].to_string()),
        None => (s, String::new()),
    };
    let nums = head
        .split('.')
        .map(|p| {
            let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().unwrap_or(0)
        })
        .collect();
    Version { nums, suffix }
}

fn chunk_cmp(a: &str, b: &str) -> Ordering {
    let split = |s: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut digit = false;
        for c in s.chars() {
            if c.is_ascii_digit() != digit && !cur.is_empty() {
                out.push(cur.clone());
                cur.clear();
            }
            digit = c.is_ascii_digit();
            cur.push(c);
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    };
    let (xs, ys) = (split(a), split(b));
    for i in 0..xs.len().max(ys.len()) {
        let x = xs.get(i);
        let y = ys.get(i);
        let ord = match (x, y) {
            (Some(x), Some(y)) => match (x.parse::<u64>(), y.parse::<u64>()) {
                (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                _ => x.to_lowercase().cmp(&y.to_lowercase()),
            },
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let n = self.nums.len().max(other.nums.len());
        for i in 0..n {
            let a = self.nums.get(i).copied().unwrap_or(0);
            let b = other.nums.get(i).copied().unwrap_or(0);
            if a != b {
                return a.cmp(&b);
            }
        }
        match (self.suffix.is_empty(), other.suffix.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => chunk_cmp(&self.suffix, &other.suffix),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn find_in_text(text: &str) -> Option<String> {
    let trimmed = {
        let lower = text.to_lowercase();
        let mut cut = text.len();
        for ext in [".zip", ".exe", ".asi", ".dll", ".7z", ".rar", ".tar", ".gz"] {
            if lower.ends_with(ext) {
                cut = text.len() - ext.len();
                break;
            }
        }
        &text[..cut]
    };
    let bytes: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = if i > 0 && (bytes[i - 1] == 'v' || bytes[i - 1] == 'V') && (i < 2 || !bytes[i - 2].is_ascii_alphanumeric()) {
            i - 1
        } else {
            i
        };
        let mut j = i;
        let mut dots = 0;
        while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == '.') {
            if bytes[j] == '.' {
                dots += 1;
            }
            j += 1;
        }
        if dots > 0 {
            let mut end = j;
            if end < bytes.len() && bytes[end] == '-' {
                let mut k = end + 1;
                while k < bytes.len() && (bytes[k].is_ascii_alphanumeric() || bytes[k] == '.' || bytes[k] == '-') {
                    k += 1;
                }
                let suffix: String = bytes[end + 1..k].iter().collect();
                if is_prerelease_text(&suffix) {
                    end = k;
                }
            }
            let mut s: String = bytes[start..end].iter().collect();
            while s.ends_with('.') || s.ends_with('-') {
                s.pop();
            }
            return Some(s.trim_start_matches(['v', 'V']).to_string());
        }
        i = j + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_releases_over_prereleases() {
        assert!(extract("v0.8.4") > extract("v0.8.3"));
        assert!(extract("v0.8.4") > extract("v0.8.4-pre"));
        assert!(extract("v0.8.4-pre") > extract("v0.8.3"));
        assert!(extract("0.3.10") > extract("0.3.4"));
        assert!(extract("1.16.0-beta.4") > extract("1.16.0-beta.2"));
        assert!(extract("1.16.0") > extract("1.16.0-beta.4"));
        assert!(extract("v0.1.8-menu-revamp-tier-presets") > extract("v0.1.7-clamp-fix-pass-presets"));
    }

    #[test]
    fn finds_versions_in_file_names() {
        assert_eq!(find_in_text("DLSS5-Feeder-1.16.0-beta.2.zip").as_deref(), Some("1.16.0-beta.2"));
        assert_eq!(find_in_text("DLSS5-Feeder-1.16.0-beta.5.zip").as_deref(), Some("1.16.0-beta.5"));
        assert!(extract(&find_in_text("DLSS5-Feeder-1.16.0-beta.5.zip").unwrap()) > extract("1.16.0-beta.2"));
        assert_eq!(find_in_text("OptiScaler-NR-v0.8.3.zip").as_deref(), Some("0.8.3"));
        assert_eq!(find_in_text("OptiScaler-DLSSNR-v0.1.8-menu-revamp-tier-presets.zip").as_deref(), Some("0.1.8"));
        assert_eq!(find_in_text("ReShade_Setup_6.8.0_Addon.exe").as_deref(), Some("6.8.0"));
        assert!(find_in_text("dlssg_for_sm86-0.3.4").is_some());
    }

    #[test]
    fn picks_newest_and_flags_prerelease_text() {
        let got = ["v0.8.1", "v0.8.4-pre", "v0.7.7"]
            .iter()
            .map(|t| extract(t))
            .max()
            .unwrap();
        assert_eq!(got, extract("v0.8.4-pre"));
        assert!(is_prerelease_text("v0.8.4-pre"));
        assert!(is_prerelease_text("1.16.0-beta.4"));
        assert!(!is_prerelease_text("v0.8.3"));
    }
}
