//! Dedup entries, filter by match, and order by (tier, match score, frecency, recency).

use crate::entry::Entry;
use crate::matcher::{KeywordPattern, Match, Pattern};
use std::collections::HashMap;

struct Candidate<'a> {
    cmd: &'a str,
    count: u32,
    last_ts: i64,
}

/// Weight of a command's most recent use. Recent commands outrank old ones
/// with the same use count.
fn recency_weight(age_secs: i64) -> f64 {
    match age_secs {
        a if a < 3_600 => 4.0,
        a if a < 86_400 => 2.0,
        a if a < 7 * 86_400 => 1.0,
        _ => 0.5,
    }
}

pub fn rank<'a>(entries: &'a [Entry], input: &str, now: i64, limit: usize) -> Vec<&'a str> {
    let pattern = Pattern::new(input);
    if pattern.is_empty() {
        return Vec::new();
    }
    let typed = input.trim();
    rank_by(entries, now, limit, |cmd| cmd != typed, |cmd| pattern.matches(cmd))
}

/// Commands containing every keyword. Earlier `wib ...` searches are skipped:
/// they contain the very keywords being searched for.
pub fn rank_keywords<'a>(entries: &'a [Entry], input: &str, now: i64, limit: usize) -> Vec<&'a str> {
    let pattern = KeywordPattern::new(input);
    if pattern.is_empty() {
        return Vec::new();
    }
    rank_by(entries, now, limit, |cmd| !is_wib_call(cmd), |cmd| pattern.matches(cmd))
}

fn is_wib_call(cmd: &str) -> bool {
    cmd.strip_prefix("wib").is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

fn rank_by(
    entries: &[Entry],
    now: i64,
    limit: usize,
    keep: impl Fn(&str) -> bool,
    matcher: impl Fn(&str) -> Option<Match>,
) -> Vec<&str> {
    if limit == 0 {
        return Vec::new();
    }

    let mut index: HashMap<&str, usize> = HashMap::with_capacity(entries.len());
    let mut candidates: Vec<Candidate> = Vec::new();
    for e in entries {
        let cmd = e.cmd.trim();
        match index.get(cmd) {
            Some(&i) => {
                let c = &mut candidates[i];
                c.count += 1;
                c.last_ts = c.last_ts.max(e.ts);
            }
            None => {
                index.insert(cmd, candidates.len());
                candidates.push(Candidate { cmd, count: 1, last_ts: e.ts });
            }
        }
    }

    let mut scored: Vec<(Match, f64, &Candidate)> = candidates
        .iter()
        .filter(|c| keep(c.cmd))
        .filter_map(|c| {
            let m = matcher(c.cmd)?;
            let frecency = c.count as f64 * recency_weight(now - c.last_ts);
            Some((m, frecency, c))
        })
        .collect();

    scored.sort_by(|(ma, fa, ca), (mb, fb, cb)| {
        ma.tier
            .cmp(&mb.tier)
            .then(mb.score.cmp(&ma.score))
            .then(fb.total_cmp(fa))
            .then(cb.last_ts.cmp(&ca.last_ts))
    });
    scored.into_iter().take(limit).map(|(_, _, c)| c.cmd).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    fn e(age: i64, cmd: &str) -> Entry {
        Entry { ts: NOW - age, cmd: cmd.into(), cwd: None, exit: None }
    }

    #[test]
    fn dedups_and_prefers_frequent() {
        let entries = vec![
            e(100, "git status"),
            e(200, "git push origin master"),
            e(300, "git push origin master"),
            e(400, "git push origin master"),
        ];
        assert_eq!(rank(&entries, "gi", NOW, 8), ["git push origin master", "git status"]);
    }

    #[test]
    fn prefix_beats_fuzzy() {
        let entries = vec![
            e(10, "docker compose up"),
            e(10, "docker compose up"),
            e(10, "dcup-tool --run"),
        ];
        // "dcup-tool" is a prefix match, so it wins despite lower frequency.
        assert_eq!(rank(&entries, "dcup", NOW, 8), ["dcup-tool --run", "docker compose up"]);
    }

    #[test]
    fn recent_beats_old_at_equal_count() {
        let entries = vec![e(30 * 86_400, "make build"), e(60, "make test")];
        assert_eq!(rank(&entries, "make", NOW, 8), ["make test", "make build"]);
    }

    #[test]
    fn keywords_rank_by_match_quality_then_frecency() {
        let entries = vec![
            e(10, "git pull origin master"),
            e(20, "docker pull nginx"),
            e(30, "docker pull nginx"),
            e(40, "git push origin master"),
            e(50, "wib pull,master"),
            e(60, "echo pullover"),
            e(70, "wibble pull"),
        ];
        assert_eq!(rank_keywords(&entries, "pull,master", NOW, 8), ["git pull origin master"]);
        assert_eq!(rank_keywords(&entries, "master pull", NOW, 8), ["git pull origin master"]);
        assert_eq!(
            rank_keywords(&entries, "pull", NOW, 8),
            ["docker pull nginx", "git pull origin master", "wibble pull", "echo pullover"]
        );
        assert_eq!(rank_keywords(&entries, "pull", NOW, 1), ["docker pull nginx"]);
        assert!(rank_keywords(&entries, " , ", NOW, 8).is_empty());
    }

    #[test]
    fn excludes_exact_input_and_respects_limit() {
        let entries = vec![e(1, "ls"), e(2, "ls -la"), e(3, "ls -lh"), e(4, "lsof -i")];
        let r = rank(&entries, "ls", NOW, 2);
        assert_eq!(r.len(), 2);
        assert!(!r.contains(&"ls"));
    }
}
