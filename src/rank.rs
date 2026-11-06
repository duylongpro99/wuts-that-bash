//! Dedup entries, filter by match, and order by (tier, match score, failed,
//! local, frecency, recency).

use crate::entry::Entry;
use crate::matcher::{KeywordPattern, Match, Pattern};
use crate::project::in_scope;
use std::collections::{HashMap, HashSet};

/// What the ranking knows about where and by whom it is asked.
#[derive(Default, Clone, Copy)]
pub struct RankContext<'a> {
    /// Project of the asking shell; commands run inside it rank first among equal matches.
    pub scope: Option<&'a str>,
    /// Commands removed with `wib forget`, compared after trimming.
    pub forgotten: Option<&'a HashSet<String>>,
    /// Never suggest a command that looks like it holds a secret.
    pub secret_guard: bool,
}

impl RankContext<'_> {
    /// No project preference and nothing forgotten.
    #[cfg(test)]
    pub fn none() -> Self {
        Self::default()
    }

    pub(crate) fn is_forgotten(&self, cmd: &str) -> bool {
        self.forgotten.is_some_and(|f| f.contains(cmd))
    }

    fn has_secret(&self, cmd: &str) -> bool {
        self.secret_guard && crate::secret::detect(cmd).is_some()
    }

    /// Never shown: forgotten, or holding a secret. `cmd` is trimmed.
    pub(crate) fn hides(&self, cmd: &str) -> bool {
        self.is_forgotten(cmd) || self.has_secret(cmd)
    }
}

/// What the entries say about one distinct (trimmed) command.
pub(crate) struct Candidate<'a> {
    pub cmd: &'a str,
    pub count: u32,
    /// Runs with no exit code: those read from `.zsh_history` only.
    pub unknown_exit: u32,
    pub last_ts: i64,
    /// Run at least once inside the asking shell's project.
    pub local: bool,
    /// At least one run has a known exit code.
    pub known_exit: bool,
    /// At least one run ended in success (or was stopped by the user).
    pub succeeded: bool,
}

impl Candidate<'_> {
    /// Known exit codes, none of them a success: a typo or a wrong flag.
    fn failed(&self) -> bool {
        self.known_exit && !self.succeeded
    }

    pub(crate) fn frecency(&self, now: i64) -> f64 {
        self.count as f64 * recency_weight(now - self.last_ts)
    }
}

/// Exit codes that do not mean the command is wrong: success, Ctrl-C (130),
/// SIGPIPE (141), SIGTERM (143) and Ctrl-Z (148).
pub(crate) fn is_ok(code: i32) -> bool {
    matches!(code, 0 | 130 | 141 | 143 | 148)
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

/// Commands matching `input` in order. `wib forget ...` is never suggested: it
/// holds the very command that was meant to disappear.
pub fn rank<'a>(entries: &'a [Entry], input: &str, now: i64, limit: usize, ctx: RankContext) -> Vec<&'a str> {
    let pattern = Pattern::new(input);
    if pattern.is_empty() {
        return Vec::new();
    }
    let typed = input.trim();
    let keep = |cmd: &str| cmd != typed && !is_wib_forget(cmd);
    rank_by(entries, now, limit, ctx, keep, |cmd| pattern.matches(cmd))
}

/// Commands containing every keyword. Earlier `wib ...` calls are skipped:
/// they contain the very keywords being searched for.
pub fn rank_keywords<'a>(
    entries: &'a [Entry],
    input: &str,
    now: i64,
    limit: usize,
    ctx: RankContext,
) -> Vec<&'a str> {
    let pattern = KeywordPattern::new(input);
    if pattern.is_empty() {
        return Vec::new();
    }
    rank_by(entries, now, limit, ctx, |cmd| wib_args(cmd).is_none(), |cmd| pattern.matches(cmd))
}

/// The arguments of a `wib` call, or `None` if `cmd` does not run wib.
pub(crate) fn wib_args(cmd: &str) -> Option<&str> {
    cmd.strip_prefix("wib")
        .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        .map(str::trim_start)
}

/// `wib forget …` / `wib unforget …`: never suggested, they hold the very
/// command that was meant to disappear (or to come back once).
fn is_wib_forget(cmd: &str) -> bool {
    wib_args(cmd).is_some_and(|args| {
        ["forget", "unforget"].iter().any(|sub| {
            args.strip_prefix(sub).is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        })
    })
}

/// One candidate per distinct trimmed command, in order of first appearance.
pub(crate) fn stats<'a>(entries: &'a [Entry], ctx: RankContext) -> Vec<Candidate<'a>> {
    let mut index: HashMap<&str, usize> = HashMap::with_capacity(entries.len());
    let mut candidates: Vec<Candidate> = Vec::new();
    for e in entries {
        let cmd = e.cmd.trim();
        let i = *index.entry(cmd).or_insert_with(|| {
            candidates.push(Candidate {
                cmd,
                count: 0,
                unknown_exit: 0,
                last_ts: e.ts,
                local: false,
                known_exit: false,
                succeeded: false,
            });
            candidates.len() - 1
        });
        let c = &mut candidates[i];
        c.count += 1;
        c.last_ts = c.last_ts.max(e.ts);
        if let (Some(scope), Some(dir)) = (ctx.scope, e.cwd.as_deref()) {
            c.local |= in_scope(dir, scope);
        }
        match e.exit {
            Some(code) => {
                c.known_exit = true;
                c.succeeded |= is_ok(code);
            }
            None => c.unknown_exit += 1,
        }
    }
    candidates
}

fn rank_by<'a>(
    entries: &'a [Entry],
    now: i64,
    limit: usize,
    ctx: RankContext,
    keep: impl Fn(&str) -> bool,
    matcher: impl Fn(&str) -> Option<Match>,
) -> Vec<&'a str> {
    if limit == 0 {
        return Vec::new();
    }
    let candidates = stats(entries, ctx);
    let mut scored: Vec<(Match, f64, &Candidate)> = candidates
        .iter()
        .filter(|c| keep(c.cmd) && !ctx.is_forgotten(c.cmd))
        .filter_map(|c| {
            let m = matcher(c.cmd)?;
            Some((m, c.frecency(now), c))
        })
        .collect();

    scored.sort_by(|(ma, fa, ca), (mb, fb, cb)| {
        ma.tier
            .cmp(&mb.tier)
            .then(mb.score.cmp(&ma.score))
            .then(ca.failed().cmp(&cb.failed()))
            .then(cb.local.cmp(&ca.local))
            .then(fb.total_cmp(fa))
            .then(cb.last_ts.cmp(&ca.last_ts))
    });
    // Scanning for secrets costs more than sorting, so only the commands about
    // to be shown are scanned.
    scored
        .into_iter()
        .map(|(_, _, c)| c.cmd)
        .filter(|cmd| !ctx.has_secret(cmd))
        .take(limit)
        .collect()
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
        assert_eq!(rank(&entries, "gi", NOW, 8, RankContext::none()), ["git push origin master", "git status"]);
    }

    #[test]
    fn prefix_beats_fuzzy() {
        let entries = vec![
            e(10, "docker compose up"),
            e(10, "docker compose up"),
            e(10, "dcup-tool --run"),
        ];
        // "dcup-tool" is a prefix match, so it wins despite lower frequency.
        assert_eq!(rank(&entries, "dcup", NOW, 8, RankContext::none()), ["dcup-tool --run", "docker compose up"]);
    }

    #[test]
    fn recent_beats_old_at_equal_count() {
        let entries = vec![e(30 * 86_400, "make build"), e(60, "make test")];
        assert_eq!(rank(&entries, "make", NOW, 8, RankContext::none()), ["make test", "make build"]);
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
        assert_eq!(rank_keywords(&entries, "pull,master", NOW, 8, RankContext::none()), ["git pull origin master"]);
        assert_eq!(rank_keywords(&entries, "master pull", NOW, 8, RankContext::none()), ["git pull origin master"]);
        assert_eq!(
            rank_keywords(&entries, "pull", NOW, 8, RankContext::none()),
            ["docker pull nginx", "git pull origin master", "wibble pull", "echo pullover"]
        );
        assert_eq!(rank_keywords(&entries, "pull", NOW, 1, RankContext::none()), ["docker pull nginx"]);
        assert!(rank_keywords(&entries, " , ", NOW, 8, RankContext::none()).is_empty());
    }

    #[test]
    fn excludes_exact_input_and_respects_limit() {
        let entries = vec![e(1, "ls"), e(2, "ls -la"), e(3, "ls -lh"), e(4, "lsof -i")];
        let r = rank(&entries, "ls", NOW, 2, RankContext::none());
        assert_eq!(r.len(), 2);
        assert!(!r.contains(&"ls"));
    }

    fn run(age: i64, cmd: &str, cwd: &str, exit: Option<i32>) -> Entry {
        Entry { ts: NOW - age, cmd: cmd.into(), cwd: Some(cwd.into()), exit }
    }

    fn scoped(scope: &str) -> RankContext<'_> {
        RankContext { scope: Some(scope), ..RankContext::none() }
    }

    #[test]
    fn project_commands_come_first_at_equal_match() {
        let mut entries = vec![run(100, "npm test", "/repo/a", Some(0)), run(90, "npm test", "/repo", Some(0))];
        entries.extend((0..10).map(|i| run(50 + i, "npm install", "/other", Some(0))));
        assert_eq!(rank(&entries, "npm", NOW, 8, scoped("/repo")), ["npm test", "npm install"]);
        assert_eq!(rank(&entries, "npm", NOW, 8, RankContext::none()), ["npm install", "npm test"]);
        // A sibling directory sharing the name prefix is not the project.
        assert_eq!(rank(&entries, "npm", NOW, 8, scoped("/re")), ["npm install", "npm test"]);
        assert_eq!(rank_keywords(&entries, "npm", NOW, 8, scoped("/repo")), ["npm test", "npm install"]);
    }

    #[test]
    fn match_quality_still_beats_project() {
        let entries = vec![run(10, "docker compose up", "/repo", Some(0)), run(10, "dcup-tool --run", "/x", Some(0))];
        assert_eq!(rank(&entries, "dcup", NOW, 8, scoped("/repo")), ["dcup-tool --run", "docker compose up"]);
    }

    #[test]
    fn never_successful_commands_sink() {
        let entries = vec![
            run(10, "gti status", "/x", Some(127)),
            run(10, "gti status", "/x", Some(127)),
            run(20, "git status", "/x", Some(0)),
        ];
        assert_eq!(rank(&entries, "g", NOW, 8, RankContext::none()), ["git status", "gti status"]);
        // Same shape, but the misspelt one is rescued by any non-failure.
        for ok in [Some(0), Some(130), Some(141), Some(143), Some(148), None] {
            let mut more = entries.clone();
            more.push(Entry { ts: NOW - 5, cmd: "gti status".into(), cwd: None, exit: ok });
            let expected = if ok.is_some() { "gti status" } else { "git status" };
            assert_eq!(rank(&more, "g", NOW, 8, RankContext::none())[0], expected, "{ok:?}");
        }
    }

    #[test]
    fn commands_without_exit_code_are_not_failed() {
        let entries = vec![
            e(10, "make lint"),
            e(10, "make lint"),
            run(20, "make test", "/x", Some(0)),
            run(30, "make bad", "/x", Some(2)),
            run(31, "make bad", "/x", Some(2)),
            run(32, "make bad", "/x", Some(2)),
        ];
        assert_eq!(rank(&entries, "make", NOW, 8, RankContext::none()), ["make lint", "make test", "make bad"]);
    }

    #[test]
    fn failure_outweighs_project() {
        let entries = vec![run(10, "cargo tset", "/repo", Some(101)), run(10, "cargo test", "/other", Some(0))];
        assert_eq!(rank(&entries, "cargo", NOW, 8, scoped("/repo")), ["cargo test", "cargo tset"]);
    }

    #[test]
    fn forgotten_commands_and_wib_forget_are_never_suggested() {
        let entries = vec![
            e(10, "git push --force"),
            e(20, "  git push --force  "),
            e(30, "git push origin"),
            e(40, "wib forget git push --force"),
            e(50, "wib forget"),
            e(60, "wib forgetful"),
            e(70, "wib unforget git push --force"),
        ];
        let forgotten: HashSet<String> = ["git push --force".to_string()].into();
        let ctx = RankContext { forgotten: Some(&forgotten), ..RankContext::none() };
        assert_eq!(rank(&entries, "git", NOW, 8, ctx), ["git push origin"]);
        assert_eq!(rank_keywords(&entries, "push", NOW, 8, ctx), ["git push origin"]);
        assert_eq!(rank(&entries, "wib", NOW, 8, ctx), ["wib forgetful"]);
        assert_eq!(rank(&entries, "push", NOW, 8, RankContext::none()), ["git push --force", "git push origin"]);
    }

    #[test]
    fn secrets_are_never_suggested_and_do_not_shrink_the_list() {
        let mut entries: Vec<Entry> = (0..5).map(|i| e(100 + i, &format!("curl -s host/{i}"))).collect();
        for _ in 0..10 {
            entries.push(e(10, "curl -H 'Authorization: Bearer abcdefghij' host"));
        }
        let guarded = RankContext { secret_guard: true, ..RankContext::none() };
        let shown = rank(&entries, "curl", NOW, 3, guarded);
        assert_eq!(shown, ["curl -s host/0", "curl -s host/1", "curl -s host/2"]);
        assert_eq!(rank(&entries, "curl", NOW, 1, RankContext::none()), ["curl -H 'Authorization: Bearer abcdefghij' host"]);
        assert!(rank_keywords(&entries, "bearer", NOW, 8, guarded).is_empty());
    }
}
