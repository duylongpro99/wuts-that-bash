//! The project a directory belongs to: the nearest ancestor holding `.git`.

use std::path::Path;

/// Root of the git project containing `cwd`, or `cwd` itself outside git.
/// `.git` may be a directory or a file (worktrees, submodules). A `.git` in
/// `home` or `/` is ignored: a dotfiles repo would otherwise make every
/// directory under it one project.
pub fn scope(cwd: &str, home: &Path) -> String {
    let start = Path::new(cwd);
    start
        .ancestors()
        .filter(|dir| *dir != home && dir.parent().is_some())
        .find(|dir| dir.join(".git").exists())
        .map_or_else(|| cwd.to_string(), |root| root.to_string_lossy().into_owned())
}

/// Is `dir` the scope directory or below it? `/repo` does not contain `/repo-old`.
pub fn in_scope(dir: &str, scope: &str) -> bool {
    match dir.strip_prefix(scope) {
        Some(rest) => rest.is_empty() || rest.starts_with('/') || scope.ends_with('/'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("wib-project-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("repo/sub/deep")).unwrap();
            TempDir(dir)
        }

        fn path(&self, rel: &str) -> String {
            self.0.join(rel).to_string_lossy().into_owned()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finds_git_directory_root() {
        let t = TempDir::new("dir");
        fs::create_dir(t.0.join("repo/.git")).unwrap();
        let home = Path::new("/nonexistent-home");
        assert_eq!(scope(&t.path("repo/sub/deep"), home), t.path("repo"));
        assert_eq!(scope(&t.path("repo"), home), t.path("repo"));
    }

    #[test]
    fn finds_git_file_root() {
        let t = TempDir::new("file");
        fs::write(t.0.join("repo/sub/.git"), "gitdir: /elsewhere\n").unwrap();
        assert_eq!(scope(&t.path("repo/sub/deep"), Path::new("/nonexistent-home")), t.path("repo/sub"));
    }

    #[test]
    fn falls_back_to_cwd_without_git() {
        let t = TempDir::new("none");
        let cwd = t.path("repo/sub");
        assert_eq!(scope(&cwd, Path::new("/nonexistent-home")), cwd);
    }

    #[test]
    fn ignores_git_in_home() {
        let t = TempDir::new("home");
        fs::create_dir(t.0.join("repo/.git")).unwrap();
        let home = t.0.join("repo");
        let cwd = t.path("repo/sub/deep");
        assert_eq!(scope(&cwd, &home), cwd);
    }

    #[test]
    fn scope_respects_directory_boundaries() {
        assert!(in_scope("/repo", "/repo"));
        assert!(in_scope("/repo/x", "/repo"));
        assert!(!in_scope("/repo-old", "/repo"));
        assert!(!in_scope("/re", "/repo"));
        assert!(in_scope("/anything", "/"));
    }
}
