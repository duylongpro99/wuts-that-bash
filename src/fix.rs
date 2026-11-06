//! "Did you mean": after "command not found" (exit 127), the same command with
//! its program name corrected to one the user really runs. Only the first
//! word changes; every other byte stays as typed, so a correction can never
//! change what a command acts on.

use crate::entry::Entry;
use crate::rank::{self, RankContext};
use std::collections::HashMap;

/// Longer commands are not worth a synchronous lookup at the prompt.
const MAX_CHARS: usize = 300;
/// A program seen only in `.zsh_history` (no exit code) counts as real after this many runs.
const MIN_HISTORY_RUNS: u32 = 3;

/// The leading program name: its byte range, if it is a plain word.
fn program(cmd: &str) -> Option<(usize, usize)> {
    let start = cmd.len() - cmd.trim_start().len();
    let end = cmd[start..].find(char::is_whitespace).map_or(cmd.len(), |i| start + i);
    let word = &cmd[start..end];
    let plain = !word.is_empty() && !word.contains(['=', '/', '\'', '"', '$', '\\', '`', '(', ')', ';', '|', '&']);
    plain.then_some((start, end))
}

/// Optimal string alignment distance over chars (Damerau–Levenshtein with
/// adjacent swaps), or `None` once it must exceed `max`.
pub fn damerau_levenshtein(a: &str, b: &str, max: usize) -> Option<usize> {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let width = b.len() + 1;
    let mut d = vec![0usize; (a.len() + 1) * width];
    for (j, cell) in d.iter_mut().take(width).enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        d[i * width] = i;
        let mut row_min = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[(i - 1) * width + j] + 1).min(d[i * width + j - 1] + 1).min(d[(i - 1) * width + j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[(i - 2) * width + j - 2] + 1);
            }
            d[i * width + j] = v;
            row_min = row_min.min(v);
        }
        if row_min > max {
            return None;
        }
    }
    Some(d[a.len() * width + b.len()]).filter(|&v| v <= max)
}

/// `python2` vs `python3`: a different version, not a typo.
fn differ_only_in_digits(a: &str, b: &str) -> bool {
    let strip = |s: &str| s.chars().filter(|c| !c.is_ascii_digit()).collect::<String>();
    strip(a) == strip(b)
}

#[derive(Default)]
struct Program {
    succeeded: bool,
    history_runs: u32,
    local: bool,
    frecency: f64,
    last_ts: i64,
}

impl Program {
    fn is_real(&self) -> bool {
        self.succeeded || self.history_runs >= MIN_HISTORY_RUNS
    }
}

/// The corrected command, or `None` when nothing close enough is known.
pub fn suggest(cmd: &str, entries: &[Entry], now: i64, ctx: RankContext) -> Option<String> {
    if cmd.contains('\n') || cmd.chars().count() > MAX_CHARS || rank::wib_args(cmd.trim_start()).is_some() {
        return None;
    }
    if ctx.secret_guard && crate::secret::detect(cmd).is_some() {
        return None;
    }
    let (start, end) = program(cmd)?;
    let typed = &cmd[start..end];

    let mut programs: HashMap<&str, Program> = HashMap::new();
    for c in rank::stats(entries, ctx) {
        if ctx.is_forgotten(c.cmd) {
            continue;
        }
        let Some((s, e)) = program(c.cmd) else { continue };
        let p = programs.entry(&c.cmd[s..e]).or_default();
        p.succeeded |= c.succeeded;
        p.history_runs += c.unknown_exit;
        p.local |= c.local;
        p.frecency += c.frecency(now);
        p.last_ts = p.last_ts.max(c.last_ts);
    }
    if programs.get(typed).is_some_and(Program::is_real) {
        return None;
    }

    let max = if typed.chars().count() <= 3 { 1 } else { 2 };
    let best = programs
        .iter()
        .filter(|(name, p)| p.is_real() && !differ_only_in_digits(typed, name))
        .filter_map(|(name, p)| Some((damerau_levenshtein(typed, name, max).filter(|&d| d > 0)?, *name, p)))
        .min_by(|(da, na, pa), (db, nb, pb)| {
            da.cmp(db)
                .then(pb.local.cmp(&pa.local))
                .then(pb.frecency.total_cmp(&pa.frecency))
                .then(pb.last_ts.cmp(&pa.last_ts))
                .then(na.cmp(nb))
        })?;
    Some(format!("{}{}{}", &cmd[..start], best.1, &cmd[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    fn ran(cmd: &str, exit: Option<i32>, cwd: &str) -> Entry {
        Entry { ts: NOW - 100, cmd: cmd.into(), cwd: Some(cwd.into()), exit }
    }

    fn ok(cmd: &str) -> Entry {
        ran(cmd, Some(0), "/x")
    }

    fn fix(cmd: &str, entries: &[Entry]) -> Option<String> {
        suggest(cmd, entries, NOW, RankContext::none())
    }

    #[test]
    fn distance() {
        assert_eq!(damerau_levenshtein("gti", "git", 2), Some(1));
        assert_eq!(damerau_levenshtein("dokcer", "docker", 2), Some(1));
        assert_eq!(damerau_levenshtein("kubeclt", "kubectl", 2), Some(1));
        assert_eq!(damerau_levenshtein("dcoker", "docker", 2), Some(1));
        assert_eq!(damerau_levenshtein("abc", "xyz", 2), None);
        assert_eq!(damerau_levenshtein("a", "abcdef", 2), None);
        assert_eq!(damerau_levenshtein("café", "cafe", 2), Some(1));
    }

    #[test]
    fn fixes_only_the_program_name() {
        let entries = [ok("git status"), ok("docker ps -a"), ok("ls -la")];
        assert_eq!(fix("gti status", &entries).as_deref(), Some("git status"));
        assert_eq!(fix("gti push origin feat-x", &entries).as_deref(), Some("git push origin feat-x"));
        assert_eq!(fix("dokcer ps", &entries).as_deref(), Some("docker ps"));
        assert_eq!(fix("sl", &entries).as_deref(), Some("ls"));
        assert_eq!(fix("  gti  'a  b'", &entries).as_deref(), Some("  git  'a  b'"));
    }

    #[test]
    fn no_suggestion_when_unsure() {
        let entries = [ok("python3 x.py"), ok("git status"), ran("make", Some(2), "/x")];
        assert_eq!(fix("python2 x", &entries), None);
        assert_eq!(fix("git sttaus", &entries), None, "the program is already real");
        assert_eq!(fix("mkae", &entries), None, "make never succeeded");
        assert_eq!(fix("zzzzzz", &entries), None);
        assert_eq!(fix("gti a\nb", &entries), None);
        assert_eq!(fix(&format!("gti {}", "x".repeat(300)), &entries), None);
        assert_eq!(fix("wib gti", &entries), None);
        assert_eq!(fix("FOO=1 gti", &entries), None);
    }

    #[test]
    fn secrets_get_no_suggestion() {
        let entries = [ok("curl -s x")];
        let guarded = RankContext { secret_guard: true, ..RankContext::none() };
        assert_eq!(suggest("crul -H 'Authorization: Bearer abcdefgh' x", &entries, NOW, guarded), None);
        assert_eq!(suggest("crul -s y", &entries, NOW, guarded).as_deref(), Some("curl -s y"));
    }

    #[test]
    fn programs_only_in_zsh_history_need_three_runs() {
        let twice = [ran("htop", None, "/"), ran("htop -d 5", None, "/")];
        assert_eq!(fix("hotp", &twice), None);
        let thrice = [ran("htop", None, "/"), ran("htop -d 5", None, "/"), ran("htop", None, "/")];
        assert_eq!(fix("hotp", &thrice).as_deref(), Some("htop"));
    }

    #[test]
    fn forgotten_commands_do_not_count() {
        let entries = [ok("git status")];
        let forgotten = ["git status".to_string()].into();
        let ctx = RankContext { forgotten: Some(&forgotten), ..RankContext::none() };
        assert_eq!(suggest("gti status", &entries, NOW, ctx), None);
    }

    #[test]
    fn project_program_comes_first() {
        let mut entries = vec![ran("make x", Some(0), "/repo")];
        entries.extend((0..5).map(|_| ran("mate y", Some(0), "/other")));
        let ctx = RankContext { scope: Some("/repo"), ..RankContext::none() };
        assert_eq!(suggest("mave", &entries, NOW, ctx).as_deref(), Some("make"));
        assert_eq!(suggest("mave", &entries, NOW, RankContext::none()).as_deref(), Some("mate"));
    }
}
