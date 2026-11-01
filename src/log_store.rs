//! wib's own append-only log: one `ts\texit\tcwd\tcmd` line per command.
//! It complements `.zsh_history`, which has no cwd/exit code and is capped by SAVEHIST.

use crate::entry::Entry;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const LOG_FILE: &str = "history.tsv";

/// `$WIB_DATA_DIR`, else `$XDG_DATA_HOME/wib`, else `~/.local/share/wib`.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = env_path("WIB_DATA_DIR") {
        return dir;
    }
    if let Some(xdg) = env_path("XDG_DATA_HOME") {
        return xdg.join("wib");
    }
    home().join(".local/share/wib")
}

pub fn home() -> PathBuf {
    env_path("HOME").unwrap_or_else(|| PathBuf::from("."))
}

pub fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

pub fn format_line(e: &Entry) -> String {
    format!(
        "{}\t{}\t{}\t{}\n",
        e.ts,
        e.exit.map(|x| x.to_string()).unwrap_or_default(),
        escape(e.cwd.as_deref().unwrap_or("")),
        escape(&e.cmd)
    )
}

pub fn parse(text: &str) -> Vec<Entry> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            let ts = parts.next()?.parse().ok()?;
            let exit = parts.next()?.parse().ok();
            let cwd = unescape(parts.next()?);
            let cmd = unescape(parts.next()?);
            if cmd.trim().is_empty() {
                return None;
            }
            Some(Entry {
                ts,
                cmd,
                cwd: (!cwd.is_empty()).then_some(cwd),
                exit,
            })
        })
        .collect()
}

pub fn load(dir: &Path) -> Vec<Entry> {
    match fs::read(dir.join(LOG_FILE)) {
        Ok(bytes) => parse(&String::from_utf8_lossy(&bytes)),
        Err(_) => Vec::new(),
    }
}

/// Should this command be recorded? Mirrors `HIST_IGNORE_SPACE`.
pub fn should_record(cmd: &str) -> bool {
    !cmd.trim().is_empty() && !cmd.starts_with(' ')
}

/// Append one entry with a single write so concurrent shells don't interleave lines.
/// The log holds command lines, so it is private to the user (0700 dir, 0600 file).
pub fn append(dir: &Path, e: &Entry) -> io::Result<()> {
    fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .mode(0o600)
        .open(dir.join(LOG_FILE))?;
    // `mode` only applies on creation; also tighten a log created by an older build.
    let meta = f.metadata()?;
    if meta.permissions().mode() & 0o077 != 0 {
        f.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    let mut line = format_line(e);
    // A previous write cut short (disk full, crash) leaves no newline; start a
    // fresh line so this entry is not glued onto the broken one.
    if ends_without_newline(&mut f)? {
        line.insert(0, '\n');
    }
    f.write_all(line.as_bytes())
}

fn ends_without_newline(f: &mut fs::File) -> io::Result<bool> {
    let len = f.metadata()?.len();
    if len == 0 {
        return Ok(false);
    }
    f.seek(SeekFrom::Start(len - 1))?;
    let mut last = [0u8; 1];
    f.read_exact(&mut last)?;
    Ok(last[0] != b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ts: i64, cmd: &str) -> Entry {
        Entry {
            ts,
            cmd: cmd.into(),
            cwd: Some("/tmp/a\tb".into()),
            exit: Some(1),
        }
    }

    #[test]
    fn round_trips_special_characters() {
        let e = entry(42, "printf 'a\\tb'\necho \"done\"\t\\");
        let parsed = parse(&format_line(&e));
        assert_eq!(parsed, vec![e]);
    }

    #[test]
    fn skips_malformed_lines() {
        let text = "x\t0\t/\tls\n100\t0\t/\t\n100\t\t\tpwd\n";
        let parsed = parse(text);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].cmd, "pwd");
        assert_eq!(parsed[0].exit, None);
        assert_eq!(parsed[0].cwd, None);
    }

    #[test]
    fn respects_hist_ignore_space() {
        assert!(should_record("ls -la"));
        assert!(!should_record(" secret-cmd"));
        assert!(!should_record("   "));
        assert!(!should_record(""));
    }

    #[test]
    fn appends_and_loads() {
        let dir = std::env::temp_dir().join(format!("wib-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        append(&dir, &entry(1, "one")).unwrap();
        append(&dir, &entry(2, "two")).unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.iter().map(|e| e.cmd.as_str()).collect::<Vec<_>>(), ["one", "two"]);
        let file_mode = fs::metadata(dir.join(LOG_FILE)).unwrap().permissions().mode() & 0o777;
        let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!((file_mode, dir_mode), (0o600, 0o700));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tightens_permissions_of_existing_log() {
        let dir = std::env::temp_dir().join(format!("wib-perm-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(LOG_FILE), "").unwrap();
        fs::set_permissions(dir.join(LOG_FILE), fs::Permissions::from_mode(0o644)).unwrap();
        append(&dir, &entry(1, "x")).unwrap();
        let mode = fs::metadata(dir.join(LOG_FILE)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn recovers_from_torn_last_line() {
        let dir = std::env::temp_dir().join(format!("wib-torn-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(LOG_FILE), "1\t0\t/\tcut sho").unwrap();
        append(&dir, &entry(2, "next")).unwrap();
        let cmds: Vec<String> = load(&dir).into_iter().map(|e| e.cmd).collect();
        assert_eq!(cmds, ["cut sho", "next"]);
        fs::remove_dir_all(&dir).unwrap();
    }
}
