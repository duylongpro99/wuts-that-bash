//! wib's own append-only log: one `ts\texit\tcwd\tcmd` line per command.
//! It complements `.zsh_history`, which has no cwd/exit code and is capped by SAVEHIST.

use crate::entry::Entry;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const LOG_FILE: &str = "history.tsv";
/// Commands removed from suggestions with `wib forget`, one escaped command per line.
pub const FORGET_FILE: &str = "forget";
const LOCK_FILE: &str = "history.lock";

/// Compaction waits until the oldest entry is this much past the cutoff, so the
/// log is rewritten about once a day rather than after every command.
const COMPACT_SLACK: i64 = 86_400;
/// How much of the log is read to find the oldest entry's timestamp.
const HEAD_BYTES: u64 = 4_096;

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

/// Append one entry with a single write, holding the log lock so concurrent
/// shells neither interleave lines nor lose them to a compaction. With
/// `keep_since`, entries older than it are then dropped once the oldest is more
/// than `COMPACT_SLACK` past it; a failed compaction leaves the log as it was.
/// The log holds command lines, so it is private to the user (0700 dir, 0600 file).
pub fn append(dir: &Path, e: &Entry, keep_since: Option<i64>) -> io::Result<()> {
    fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    // Without a lock (a filesystem that cannot lock, a lock file owned by
    // another user) still record the command, but skip the rewrite.
    let lock = lock(dir).ok();
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
    f.write_all(line.as_bytes())?;
    drop(f);
    if let (Some(cutoff), Some(_held)) = (keep_since, &lock) {
        let _ = compact_if_stale(dir, cutoff);
    }
    Ok(())
}

/// Commands removed with `wib forget`, trimmed. A missing file means none.
pub fn load_forgotten(dir: &Path) -> HashSet<String> {
    match fs::read(dir.join(FORGET_FILE)) {
        Ok(bytes) => String::from_utf8_lossy(&bytes)
            .lines()
            .map(|line| unescape(line).trim().to_string())
            .filter(|cmd| !cmd.is_empty())
            .collect(),
        Err(_) => HashSet::new(),
    }
}

/// Add `cmd` to the forget file. Returns `false` if it was already there.
/// Like the log, the file holds command lines and stays private (0700 dir, 0600 file).
pub fn forget(dir: &Path, cmd: &str) -> io::Result<bool> {
    let cmd = cmd.trim();
    fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    let _lock = lock(dir).ok();
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .mode(0o600)
        .open(dir.join(FORGET_FILE))?;
    if f.metadata()?.permissions().mode() & 0o077 != 0 {
        f.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    if load_forgotten(dir).contains(cmd) {
        return Ok(false);
    }
    let mut line = escape(cmd) + "\n";
    if ends_without_newline(&mut f)? {
        line.insert(0, '\n');
    }
    f.write_all(line.as_bytes())?;
    Ok(true)
}

/// Exclusive lock shared by every writer. It lives in its own file because
/// compaction replaces `history.tsv`, and a lock on the replaced file would no
/// longer keep a waiting writer from appending to it. Released on drop.
fn lock(dir: &Path) -> io::Result<fs::File> {
    let f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(dir.join(LOCK_FILE))?;
    f.lock()?;
    Ok(f)
}

fn line_ts(line: &[u8]) -> Option<i64> {
    let tab = line.iter().position(|&b| b == b'\t')?;
    std::str::from_utf8(&line[..tab]).ok()?.parse().ok()
}

/// Drop entries older than `cutoff`, keeping the others byte for byte. Callers
/// hold the log lock. The new log is written beside the old one and renamed
/// over it, so a concurrent reader sees either the old or the new log.
fn compact_if_stale(dir: &Path, cutoff: i64) -> io::Result<()> {
    let path = dir.join(LOG_FILE);
    let mut head = Vec::new();
    fs::File::open(&path)?.take(HEAD_BYTES).read_to_end(&mut head)?;
    // Commands are appended in order, so the first entry is the oldest. If the
    // head holds no complete entry, fall through and let the full pass decide.
    let oldest = head.split(|&b| b == b'\n').find_map(line_ts);
    if oldest.is_some_and(|ts| ts >= cutoff.saturating_sub(COMPACT_SLACK)) {
        return Ok(());
    }

    let raw = fs::read(&path)?;
    let mut kept = Vec::with_capacity(raw.len());
    let mut dropped = false;
    for line in raw.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        if line_ts(line).is_some_and(|ts| ts >= cutoff) {
            kept.extend_from_slice(line);
            kept.push(b'\n');
        } else {
            dropped = true;
        }
    }
    if !dropped {
        return Ok(());
    }

    let owner = fs::metadata(&path)?;
    let tmp = dir.join(format!("{LOG_FILE}.tmp"));
    let written = (|| {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        // Keep the log's owner: a root shell sharing $HOME must not leave the
        // user a log they can no longer write. Fails, and so skips, for anyone
        // but the owner or root.
        std::os::unix::fs::fchown(&f, Some(owner.uid()), Some(owner.gid()))?;
        f.write_all(&kept)?;
        f.sync_all()?;
        fs::rename(&tmp, &path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
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
        append(&dir, &entry(1, "one"), None).unwrap();
        append(&dir, &entry(2, "two"), None).unwrap();
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
        append(&dir, &entry(1, "x"), None).unwrap();
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
        append(&dir, &entry(2, "next"), None).unwrap();
        let cmds: Vec<String> = load(&dir).into_iter().map(|e| e.cmd).collect();
        assert_eq!(cmds, ["cut sho", "next"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    fn fresh_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("wib-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compaction_drops_old_entries_and_keeps_others_verbatim() {
        let dir = fresh_dir("compact");
        let day = 86_400;
        let text = format!(
            "{}\t0\t/\told\nbroken line\n{}\t1\t/tmp/a\\tb\tprintf 'x\\ny'\n{}\t\t\tnew\n",
            10 * day,
            40 * day,
            41 * day
        );
        fs::write(dir.join(LOG_FILE), &text).unwrap();
        fs::set_permissions(dir.join(LOG_FILE), fs::Permissions::from_mode(0o600)).unwrap();
        append(&dir, &entry(42 * day, "latest"), Some(30 * day)).unwrap();
        let raw = fs::read_to_string(dir.join(LOG_FILE)).unwrap();
        let expected_tail = format_line(&entry(42 * day, "latest"));
        assert_eq!(
            raw,
            format!("{}\t1\t/tmp/a\\tb\tprintf 'x\\ny'\n{}\t\t\tnew\n{expected_tail}", 40 * day, 41 * day)
        );
        let mode = fs::metadata(dir.join(LOG_FILE)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert!(!dir.join(format!("{LOG_FILE}.tmp")).exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn compaction_waits_for_the_slack() {
        let dir = fresh_dir("slack");
        let day = 86_400;
        // The oldest entry is past the cutoff, but by less than a day.
        append(&dir, &entry(29 * day + 10, "slightly old"), None).unwrap();
        let before = fs::read_to_string(dir.join(LOG_FILE)).unwrap();
        compact_if_stale(&dir, 30 * day).unwrap();
        assert_eq!(fs::read_to_string(dir.join(LOG_FILE)).unwrap(), before);
        compact_if_stale(&dir, 31 * day).unwrap();
        assert_eq!(fs::read_to_string(dir.join(LOG_FILE)).unwrap(), "");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn forget_dedups_and_round_trips_multiline() {
        let dir = std::env::temp_dir().join(format!("wib-forget-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(load_forgotten(&dir).is_empty());
        assert!(forget(&dir, " git push --force ").unwrap());
        assert!(!forget(&dir, "git push --force").unwrap());
        assert!(forget(&dir, "a\tb\\c\nd").unwrap());
        let raw = fs::read_to_string(dir.join(FORGET_FILE)).unwrap();
        assert_eq!(raw.lines().count(), 2);
        let expected: HashSet<String> = ["git push --force".to_string(), "a\tb\\c\nd".to_string()].into();
        assert_eq!(load_forgotten(&dir), expected);
        let file_mode = fs::metadata(dir.join(FORGET_FILE)).unwrap().permissions().mode() & 0o777;
        let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!((file_mode, dir_mode), (0o600, 0o700));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_compaction_without_a_period() {
        let dir = fresh_dir("noperiod");
        append(&dir, &entry(1, "ancient"), None).unwrap();
        append(&dir, &entry(2, "also ancient"), None).unwrap();
        assert_eq!(load(&dir).len(), 2);
        fs::remove_dir_all(&dir).unwrap();
    }
}
