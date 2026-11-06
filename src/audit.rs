//! `wib audit`: find commands holding secrets in `.zsh_history`, wib's log and
//! the forget file, show them masked, and with `--fix` remove them from wib's
//! own files. `.zsh_history` belongs to zsh and is never written.

use crate::log_store::{self, FORGET_FILE, LOG_FILE};
use crate::secret::{self, Finding};
use std::fs;
use std::path::{Path, PathBuf};

/// `cmd` with every secret value replaced by `…`. A token whose prefix is the
/// same for everyone (`ghp_`) keeps that prefix so the user can tell which
/// token it was; nothing else of a value is shown, however short.
pub fn mask(cmd: &str, findings: &[Finding]) -> String {
    let mut out = String::with_capacity(cmd.len());
    let mut last = 0;
    for f in findings {
        out.push_str(&cmd[last..f.span.start]);
        if let Some(prefix) = f.kind.public_prefix().filter(|p| cmd[f.span.clone()].starts_with(p)) {
            out.push_str(prefix);
        }
        out.push('…');
        last = f.span.end;
    }
    out.push_str(&cmd[last..]);
    out
}

/// `cmd` masked, as one terminal line.
pub fn shown(cmd: &str) -> String {
    crate::printable(&mask(cmd, &secret::detect_all(cmd)))
}

struct Hit {
    file: String,
    line: usize,
    labels: String,
    masked: String,
}

fn hit(file: &str, line: usize, cmd: &str) -> Option<Hit> {
    let findings = secret::detect_all(cmd);
    if findings.is_empty() {
        return None;
    }
    let mut labels: Vec<&str> = findings.iter().map(|f| f.kind.label()).collect();
    labels.dedup();
    Some(Hit {
        file: file.to_string(),
        line,
        labels: labels.join(", "),
        masked: crate::printable(&mask(cmd, &findings)),
    })
}

/// `~/x` for a path under the home directory.
fn display(path: &Path) -> String {
    let home = log_store::home();
    match path.strip_prefix(&home) {
        Ok(rest) if home != Path::new(".") => format!("~/{}", rest.display()),
        _ => path.display().to_string(),
    }
}

/// Is this line of wib's log a command holding a secret?
fn log_line_has_secret(line: &str) -> bool {
    log_store::parse(line).iter().any(|e| secret::detect(&e.cmd).is_some())
}

fn forget_line_has_secret(line: &str) -> bool {
    secret::detect(&log_store::unescape(line)).is_some()
}

/// Scan every source and print what was found. Returns whether anything was.
pub fn run(zsh_history: &Path, fix: bool) -> Result<bool, String> {
    let dir = log_store::data_dir();
    let mut hits = Vec::new();

    let zsh_name = display(zsh_history);
    if let Ok(raw) = fs::read(zsh_history) {
        for (line, e) in crate::zsh_history::parse_with_lines(&raw) {
            hits.extend(hit(&zsh_name, line, &e.cmd));
        }
    }
    let zsh_hits = hits.len();
    let read_lines = |name: &str| -> Vec<String> {
        fs::read(dir.join(name))
            .map(|b| String::from_utf8_lossy(&b).lines().map(str::to_string).collect())
            .unwrap_or_default()
    };
    let log_name = display(&dir.join(LOG_FILE));
    for (i, line) in read_lines(LOG_FILE).iter().enumerate() {
        if let Some(e) = log_store::parse(line).first() {
            hits.extend(hit(&log_name, i + 1, &e.cmd));
        }
    }
    let forget_name = display(&dir.join(FORGET_FILE));
    for (i, line) in read_lines(FORGET_FILE).iter().enumerate() {
        hits.extend(hit(&forget_name, i + 1, &log_store::unescape(line)));
    }

    if hits.is_empty() {
        println!("no secrets found");
        return Ok(false);
    }
    let width = hits.iter().map(|h| h.file.len() + 1 + h.line.to_string().len()).max().unwrap_or(0);
    let label_width = hits.iter().map(|h| h.labels.chars().count()).max().unwrap_or(0);
    for h in &hits {
        let place = format!("{}:{}", h.file, h.line);
        println!("{place:<width$}   {:<label_width$}  {}", h.labels, h.masked);
    }
    let n = hits.len();
    let (noun, verb) = if n == 1 { ("command looks", "it contains a secret") } else { ("commands look", "they contain secrets") };
    if !fix {
        println!("{n} {noun} like {verb}. Rotate them, then run `wib audit --fix`.");
        return Ok(true);
    }

    let removed = remove_secret_lines(&dir)?;
    for (name, count) in removed {
        println!("removed {count} line{} from {}", if count == 1 { "" } else { "s" }, display(&dir.join(name)));
    }
    if zsh_hits > 0 {
        let lines = if zsh_hits == 1 { "1 line".to_string() } else { format!("{zsh_hits} lines") };
        println!(
            "{lines} in {zsh_name} still contain secrets; wib never edits that file. Close your other shells, \
             delete the listed lines in an editor, then open a new terminal (a running zsh keeps its history \
             in memory and may write it back)."
        );
    }
    Ok(true)
}

/// Remove lines holding secrets from wib's log and forget file, reading them
/// again while holding the lock so a command recorded meanwhile is kept.
///
/// With the guard off, a forgotten command is hidden only by the forget file
/// (it may still be in `.zsh_history`), so the forget file is left alone.
/// A file in the data directory and the test for a line in it that holds a secret.
type SecretFile = (&'static str, fn(&str) -> bool);

fn remove_secret_lines(dir: &Path) -> Result<Vec<(&'static str, usize)>, String> {
    let mut files: Vec<SecretFile> = vec![(LOG_FILE, log_line_has_secret)];
    if secret::guard_enabled() {
        files.push((FORGET_FILE, forget_line_has_secret));
    } else {
        println!(
            "kept {}: with WIB_SECRET_GUARD=0, removing a forgotten command would suggest it again",
            display(&dir.join(FORGET_FILE))
        );
    }
    if !files.iter().any(|(name, _)| dir.join(name).exists()) {
        return Ok(Vec::new());
    }
    let held = log_store::lock(dir).map_err(|e| format!("cannot lock {}: {e}", dir.display()))?;
    let mut removed = Vec::new();
    for (name, has_secret) in files.iter().copied() {
        let path: PathBuf = dir.join(name);
        let Ok(raw) = fs::read(&path) else { continue };
        let mut kept = Vec::with_capacity(raw.len());
        let mut count = 0;
        for line in raw.split_inclusive(|&b| b == b'\n') {
            let text = String::from_utf8_lossy(line);
            if has_secret(text.trim_end_matches('\n')) {
                count += 1;
            } else {
                kept.extend_from_slice(line);
            }
        }
        if count > 0 {
            log_store::rewrite_private(dir, name, &kept, &held)
                .map_err(|e| format!("cannot rewrite {}: {e}", path.display()))?;
            removed.push((name, count));
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every substring of `secret` of length 3 that `masked` must not contain.
    fn assert_hidden(masked: &str, secret: &str) {
        let chars: Vec<char> = secret.chars().collect();
        for w in chars.windows(3) {
            let part: String = w.iter().collect();
            assert!(!masked.contains(&part), "{masked:?} shows {part:?} of {secret:?}");
        }
    }

    fn masked(cmd: &str) -> String {
        mask(cmd, &secret::detect_all(cmd))
    }

    #[test]
    fn masks_every_secret_in_a_command() {
        let cmd = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMIK7MDENG aws s3 ls";
        let out = masked(cmd);
        assert_eq!(out, "AWS_ACCESS_KEY_ID=AKIA… AWS_SECRET_ACCESS_KEY=… aws s3 ls");
        assert_hidden(&out, "IOSFODNN7EXAMPLE");
        assert_hidden(&out, "wJalrXUtnFEMIK7MDENG");
    }

    #[test]
    fn short_secrets_are_hidden_entirely() {
        let out = masked("tool --password abcd");
        assert_eq!(out, "tool --password …");
        assert_hidden(&out, "abcd");
    }

    #[test]
    fn url_password_and_multibyte_text() {
        let out = masked("psql postgres://app:mậtkhẩu99@db.local/app # «chú thích»");
        assert_eq!(out, "psql postgres://app:…@db.local/app # «chú thích»");
        assert_hidden(&out, "mậtkhẩu99");
    }

    #[test]
    fn known_token_keeps_only_its_prefix() {
        let token = "ghp_0123456789abcdefghijABCDEFGHIJ012345";
        let out = masked(&format!("export GITHUB_TOKEN={token}"));
        assert_eq!(out, "export GITHUB_TOKEN=ghp_…");
        assert_hidden(&out, &token[4..]);
    }

    #[test]
    fn shown_is_one_terminal_line() {
        assert_eq!(shown("TOKEN=abc123 run\n\x1b[2J"), "TOKEN=… run\\n\\x1b[2J");
    }
}
