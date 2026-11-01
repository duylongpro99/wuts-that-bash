//! Parser for zsh `EXTENDED_HISTORY` files (`: <ts>:<duration>;<command>`).

use crate::entry::Entry;

const META: u8 = 0x83;

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

/// Parse `: 1728360793:0;git status` into (ts, "git status").
fn parse_header(line: &str) -> Option<(i64, &str)> {
    let rest = line.strip_prefix(": ")?;
    let colon = rest.find(':')?;
    let ts: i64 = rest[..colon].parse().ok()?;
    let rest = &rest[colon + 1..];
    let semi = rest.find(';')?;
    if !rest[..semi].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((ts, &rest[semi + 1..]))
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
    let unmetafied;
    let bytes = if raw.contains(&META) {
        unmetafied = unmetafy(raw);
        &unmetafied[..]
    } else {
        raw
    };
    let text = String::from_utf8_lossy(bytes);
    let mut entries: Vec<Entry> = Vec::new();
    // True while the previous line ended with a continuation backslash.
    let mut continuing = false;
    // Whether the entry being continued was kept (inside the period).
    let mut keeping = false;

    for line in text.split('\n') {
        if continuing {
            continuing = line.ends_with('\\');
            if keeping {
                if let Some(last) = entries.last_mut() {
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
                entries.push(Entry {
                    ts,
                    cmd: cmd.to_string(),
                    cwd: None,
                    exit: None,
                });
            }
        }
    }

    for e in &mut entries {
        let trimmed = e.cmd.trim_end();
        if trimmed.len() != e.cmd.len() {
            e.cmd.truncate(trimmed.len());
        }
    }
    entries.retain(|e| !e.cmd.trim().is_empty());
    entries
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
}
