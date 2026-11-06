//! Parser for zsh `EXTENDED_HISTORY` files (`: <ts>:<duration>;<command>`).

use crate::entry::Entry;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const META: u8 = 0x83;

/// First tail read; doubled until it reaches a command older than the period.
const INITIAL_CHUNK: u64 = 256 * 1024;

/// With `sharehistory`, zsh writes a command when it finishes but stamps it with
/// its start time, so a long-running command can land after newer ones. Reading
/// back stops only at a command this much older than the cutoff.
const DISORDER_SLACK: i64 = 86_400;

/// zsh stores some bytes (including most non-ASCII UTF-8 bytes) "metafied":
/// the byte `b` is written as `0x83, b ^ 0x20`. Undo that.
pub fn unmetafy(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut iter = bytes.iter();
    while let Some(&b) = iter.next() {
        if b == META {
            if let Some(&next) = iter.next() {
                out.push(next ^ 0x20);
            }
        } else {
            out.push(b);
        }
    }
    out
}

/// Split `: 1728360793:0;git status` into the timestamp and the byte offset of
/// the command.
fn split_header(line: &[u8]) -> Option<(i64, usize)> {
    let rest = line.strip_prefix(b": ")?;
    let colon = rest.iter().position(|&b| b == b':')?;
    let ts: i64 = std::str::from_utf8(&rest[..colon]).ok()?.parse().ok()?;
    let after_ts = &rest[colon + 1..];
    let semi = after_ts.iter().position(|&b| b == b';')?;
    if !after_ts[..semi].iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some((ts, 2 + colon + 1 + semi + 1))
}

/// Parse `: 1728360793:0;git status` into (ts, "git status").
fn parse_header(line: &str) -> Option<(i64, &str)> {
    split_header(line.as_bytes()).map(|(ts, at)| (ts, &line[at..]))
}

/// Parse raw `.zsh_history` bytes. Lines without an extended header are
/// skipped; multi-line commands (lines ending in `\`) are joined with `\n`.
#[cfg(test)]
pub fn parse(raw: &[u8]) -> Vec<Entry> {
    parse_since(raw, i64::MIN)
}

/// Like `parse`, but entries older than `cutoff` are skipped before any
/// allocation, which keeps per-keystroke queries cheap on long histories.
pub fn parse_since(raw: &[u8], cutoff: i64) -> Vec<Entry> {
    parse_lines(raw, cutoff).into_iter().map(|(_, e)| e).collect()
}

/// Every entry with the 1-based line number of its header, for pointing the
/// user at a line of the file.
pub fn parse_with_lines(raw: &[u8]) -> Vec<(usize, Entry)> {
    parse_lines(raw, i64::MIN)
}

fn parse_lines(raw: &[u8], cutoff: i64) -> Vec<(usize, Entry)> {
    let unmetafied;
    let bytes = if raw.contains(&META) {
        unmetafied = unmetafy(raw);
        &unmetafied[..]
    } else {
        raw
    };
    let text = String::from_utf8_lossy(bytes);
    let mut entries: Vec<(usize, Entry)> = Vec::new();
    // True while the previous line ended with a continuation backslash.
    let mut continuing = false;
    // Whether the entry being continued was kept (inside the period).
    let mut keeping = false;

    // Cutting at `\n` keeps line numbers: a metafied byte is never `\n`.
    for (n, line) in text.split('\n').enumerate() {
        if continuing {
            continuing = line.ends_with('\\');
            if keeping {
                if let Some((_, last)) = entries.last_mut() {
                    last.cmd.pop(); // drop the trailing backslash
                    last.cmd.push('\n');
                    last.cmd.push_str(line);
                }
            }
            continue;
        }
        if let Some((ts, cmd)) = parse_header(line) {
            continuing = cmd.ends_with('\\');
            keeping = ts >= cutoff;
            if keeping {
                entries.push((
                    n + 1,
                    Entry {
                        ts,
                        cmd: cmd.to_string(),
                        cwd: None,
                        exit: None,
                    },
                ));
            }
        }
    }

    for (_, e) in &mut entries {
        let trimmed = e.cmd.trim_end();
        if trimmed.len() != e.cmd.len() {
            e.cmd.truncate(trimmed.len());
        }
    }
    entries.retain(|(_, e)| !e.cmd.trim().is_empty());
    entries
}

/// Entries from the history file at `path` that ran at or after `cutoff`.
/// Reads the file from its end and stops once it reaches commands older than
/// the cutoff, so the cost follows the period rather than the file size.
pub fn read_since(path: &Path, cutoff: i64) -> io::Result<Vec<Entry>> {
    read_since_chunked(path, cutoff, INITIAL_CHUNK)
}

fn read_since_chunked(path: &Path, cutoff: i64, initial_chunk: u64) -> io::Result<Vec<Entry>> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut chunk = initial_chunk.max(1);
    loop {
        let start = len.saturating_sub(chunk);
        let mut raw = Vec::with_capacity((len - start) as usize);
        f.seek(SeekFrom::Start(start))?;
        // `take`: zsh may rewrite the file meanwhile; parse whatever is there.
        (&mut f).take(len - start).read_to_end(&mut raw)?;
        if start == 0 {
            return Ok(parse_since(&raw, cutoff));
        }
        if let Some(at) = first_header_if_older(&raw, cutoff.saturating_sub(DISORDER_SLACK)) {
            return Ok(parse_since(&raw[at..], cutoff));
        }
        chunk = chunk.saturating_mul(2);
    }
}

/// Offset of the first complete header line in a tail chunk, if that command is
/// older than `limit`. The first line is skipped because the chunk may start
/// mid-line. Cutting at `\n` is safe: a metafied byte is never `\n`. A line
/// after one ending in `\\` may be part of a multi-line command that merely
/// looks like a header (a heredoc of history), so it is not trusted.
fn first_header_if_older(raw: &[u8], limit: i64) -> Option<usize> {
    let mut at = raw.iter().position(|&b| b == b'\n')? + 1;
    while at < raw.len() {
        // When the previous line's end is outside the chunk, assume it continues.
        let continues = at < 2 || raw[at - 2] == b'\\';
        let end = raw[at..].iter().position(|&b| b == b'\n').map_or(raw.len(), |i| at + i);
        if !continues {
            if let Some((ts, _)) = split_header(&raw[at..end]) {
                return (ts < limit).then_some(at);
            }
        }
        at = end + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmetafies_non_ascii() {
        // "é" is 0xC3 0xA9; zsh metafies 0xA9 (>= 0x83) as 0x83 0x89.
        let raw = [b'a', 0xC3, META, 0xA9 ^ 0x20, b'b'];
        assert_eq!(unmetafy(&raw), "aéb".as_bytes());
    }

    #[test]
    fn parses_extended_entries() {
        let raw = b": 100:0;git status\n: 200:3;docker compose up -d\n";
        let e = parse(raw);
        assert_eq!(e.len(), 2);
        assert_eq!((e[0].ts, e[0].cmd.as_str()), (100, "git status"));
        assert_eq!((e[1].ts, e[1].cmd.as_str()), (200, "docker compose up -d"));
    }

    #[test]
    fn joins_multiline_commands() {
        let raw = b": 100:0;for f in *; do\\\n  echo $f\\\ndone\n: 101:0;ls\n";
        let e = parse(raw);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].cmd, "for f in *; do\n  echo $f\ndone");
        assert_eq!(e[1].cmd, "ls");
    }

    #[test]
    fn reports_header_line_numbers() {
        // "é" metafied, and a multi-line command spanning lines 2-3.
        let raw = [
            b": 100:0;echo caf\xC3".as_slice(),
            &[META, 0xA9 ^ 0x20],
            b"\nplain\n: 101:0;for x; do\\\ndone\n: 102:0;ls\n",
        ]
        .concat();
        let found: Vec<(usize, String)> = parse_with_lines(&raw).into_iter().map(|(n, e)| (n, e.cmd)).collect();
        assert_eq!(found, [(1, "echo café".to_string()), (3, "for x; do\ndone".to_string()), (5, "ls".to_string())]);
    }

    #[test]
    fn skips_broken_and_plain_lines() {
        let raw = b"plain line\n: abc:0;bad ts\n: 100:x;bad dur\n: 100:0;ok\n:\n\n: 101:0;   \n";
        let e = parse(raw);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].cmd, "ok");
    }

    #[test]
    fn skips_entries_before_cutoff_including_continuations() {
        let raw = b": 100:0;old one\\\nstill old\n: 200:0;new\\\nline two\n: 300:0;newest\n";
        let e = parse_since(raw, 150);
        let cmds: Vec<&str> = e.iter().map(|e| e.cmd.as_str()).collect();
        assert_eq!(cmds, ["new\nline two", "newest"]);
    }

    #[test]
    fn parses_fixture_file() {
        let raw = include_bytes!("../tests/fixtures/zsh_history_sample");
        let e = parse(raw);
        let cmds: Vec<&str> = e.iter().map(|e| e.cmd.as_str()).collect();
        assert!(cmds.contains(&"git push origin master"));
        assert!(cmds.contains(&"echo \"xin chào\""));
        assert!(cmds.contains(&"for f in *.log; do\n  wc -l $f\ndone"));
        assert!(!cmds.iter().any(|c| c.contains("not an entry")));
    }

    fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("wib-tail-{name}-{}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// 300 commands, 1000s apart, with multi-line and metafied non-ASCII
    /// commands and timestamps out of order by less than `DISORDER_SLACK`.
    fn mixed_history() -> Vec<u8> {
        let mut raw = Vec::new();
        for i in 0..300i64 {
            let ts = i * 1000 + if i % 5 == 0 { 40_000 } else { 0 };
            raw.extend_from_slice(format!(": {ts}:0;").as_bytes());
            match i % 3 {
                0 => raw.extend_from_slice(format!("for f in {i}; do\\\n  echo $f\\\ndone").as_bytes()),
                1 => {
                    raw.extend_from_slice(format!("echo caf{i}").as_bytes());
                    raw.extend_from_slice(&[0xC3, META, 0xA9 ^ 0x20]);
                }
                _ => raw.extend_from_slice(format!("git status {i}").as_bytes()),
            }
            raw.push(b'\n');
        }
        raw
    }

    #[test]
    fn tail_read_matches_full_parse() {
        let fixture: &[u8] = include_bytes!("../tests/fixtures/zsh_history_sample");
        let cases: [(&str, Vec<u8>); 4] = [
            ("fixture", fixture.to_vec()),
            ("mixed", mixed_history()),
            ("empty", Vec::new()),
            ("plain", b"no header here\nnor here\n".to_vec()),
        ];
        for (name, raw) in cases {
            let path = temp_file(name, &raw);
            for cutoff in [i64::MIN, 0, 150_000, 200_000, 299_000, i64::MAX] {
                let full = parse_since(&raw, cutoff);
                for chunk in [1, 7, 64, 1024, INITIAL_CHUNK] {
                    let tail = read_since_chunked(&path, cutoff, chunk).unwrap();
                    assert_eq!(tail, full, "{name}: cutoff {cutoff}, chunk {chunk}");
                }
            }
            std::fs::remove_file(&path).unwrap();
        }
    }

    #[test]
    fn tail_read_stops_at_old_commands() {
        let raw = b"garbage\n: 100:0;old\n: 500000:0;new\n";
        // The chunk starts inside `garbage`; the first header is old enough.
        assert_eq!(first_header_if_older(&raw[3..], 300_000), Some(5));
        // A recent first header means the reader must look further back.
        assert_eq!(first_header_if_older(&raw[18..], 300_000), None);
    }

    #[test]
    fn tail_read_ignores_header_lookalikes_inside_commands() {
        // A recent heredoc whose body looks like an old history entry.
        let mut raw = b": 100:0;ancient\n".to_vec();
        raw.extend_from_slice(b": 900000:0;cat <<X\\\n: 100:0;line\\\nX\n: 900001:0;ls\n");
        let path = temp_file("lookalike", &raw);
        let full = parse_since(&raw, 800_000);
        assert_eq!(full.len(), 2);
        for chunk in 1..raw.len() as u64 {
            assert_eq!(read_since_chunked(&path, 800_000, chunk).unwrap(), full, "chunk {chunk}");
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn tail_read_reports_missing_file() {
        let path = std::env::temp_dir().join("wib-tail-missing-file");
        assert!(read_since(&path, 0).is_err());
    }
}
