use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

pub fn norm_filename(input: &str) -> PathBuf {
    let expanded = match shellexpand::tilde(input) {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    };

    let unescaped = expanded.replace(r"\ ", " ");
    let unescaped = unescape::unescape(&unescaped).unwrap_or(expanded);

    let sanitized_chars: String = unescaped
        .chars()
        .filter(|&c| {
            if c.is_control() {
                return false;
            }
            match c {
                '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\0' => false,
                _ => true,
            }
        })
        .collect();

    let path = PathBuf::from(sanitized_chars);
    clean_path(&path)
}

fn clean_path(path: &Path) -> PathBuf {
    if let Ok(canonical_path) = std::fs::canonicalize(path) {
        return canonical_path;
    }

    let mut out = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                // Look at the last component we added
                match out.components().next_back() {
                    None | Some(Component::ParentDir) => {
                        out.push(Component::ParentDir.as_os_str());
                    }
                    Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                    Some(Component::Normal(_)) => {
                        out.pop();
                    }
                    Some(Component::CurDir) => {}
                }
            }
            Component::Normal(c) => out.push(c),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::Prefix(p) => out.push(p.as_os_str()),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_sanitize() {
        let input1 = r"new\ file.txt";
        assert_eq!(norm_filename(input1).to_str(), Some("new file.txt"));

        let input2 = "~/documents/secret\u{0007}file*.txt";
        let expected = dirs::home_dir()
            .map(|home| home.join("documents/secretfile.txt"))
            .unwrap();
        assert_eq!(norm_filename(input2).to_str(), expected.to_str(),);
    }

    #[test]
    fn test_path_cleansing() {
        let input1 = "../file.txt";
        assert_eq!(norm_filename(input1).to_str(), Some("../file.txt"));

        let input2 = "/home/pablo/../../home/pablo/../pablo/./././../pablo/./fil\0e.\\.txt";
        assert_eq!(
            norm_filename(input2).to_str(),
            Some(r"/home/pablo/file.\.txt")
        );

        let input3 = "../.././../doc.txt";
        assert_eq!(norm_filename(input3).to_str(), Some(r"../../../doc.txt"));
    }
}
