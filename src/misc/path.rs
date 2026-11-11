use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

pub fn norm_filename(input: &str) -> PathBuf {
    let expanded = match shellexpand::tilde(input) {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    };

    let unescaped: String = expanded
        .chars()
        .filter(|&ch| !ch.is_control() && ch != '\\')
        .collect();
    let path = PathBuf::from(unescaped);
    clean_path(&path)
}

fn clean_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir) => {}
                _ => out.push(Component::ParentDir.as_os_str()),
            },
            Component::Normal(c) => out.push(c),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::Prefix(p) => out.push(p.as_os_str()),
        }
    }

    if out.as_os_str().is_empty() {
        return PathBuf::from(".");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_filename_unescapes_backslashes() {
        assert_eq!(
            norm_filename(r"new\ file.txt"),
            PathBuf::from("new file.txt")
        );
        assert_eq!(norm_filename(r"\a"), PathBuf::from("a"));
        assert_eq!(norm_filename(r"\\|"), PathBuf::from("|"));
    }

    #[test]
    fn norm_filename_expands_tilde_and_removes_control_characters() {
        let home = dirs::home_dir().expect("home directory should be available");
        let actual = norm_filename("~/documents/secret\u{0007}file.txt");

        assert_eq!(actual, home.join("documents/secretfile.txt"));
    }

    #[test]
    fn clean_path_normalizes_components() {
        assert_eq!(clean_path(Path::new("a/./../b")), PathBuf::from("b"));
        assert_eq!(
            clean_path(Path::new("../.././../doc.txt")),
            PathBuf::from("../../../doc.txt")
        );
        assert_eq!(clean_path(Path::new("a/..")), PathBuf::from("."));
        assert_eq!(clean_path(Path::new("/../../a")), PathBuf::from("/a"));
    }
}
