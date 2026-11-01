//! Match a typed input against a command: prefix first, then fuzzy subsequence.

/// Lower tier sorts first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Prefix = 0,
    Fuzzy = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub tier: Tier,
    /// Higher is better; only meaningful within the fuzzy tier.
    pub score: i32,
}

/// Input chars, pre-folded once per query.
pub struct Pattern {
    chars: Vec<char>,
    case_sensitive: bool,
}

impl Pattern {
    /// Smart case: an input containing an uppercase letter matches case-sensitively.
    pub fn new(input: &str) -> Self {
        let case_sensitive = input.chars().any(char::is_uppercase);
        let chars = input.chars().map(|c| fold(c, case_sensitive)).collect();
        Pattern { chars, case_sensitive }
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn matches(&self, cmd: &str) -> Option<Match> {
        if self.chars.is_empty() {
            return None;
        }
        // Cheap prefix test first: the common case for typed commands.
        let mut cmd_chars = cmd.chars();
        if self
            .chars
            .iter()
            .all(|&p| cmd_chars.next().map(|c| fold(c, self.case_sensitive)) == Some(p))
        {
            return Some(Match { tier: Tier::Prefix, score: 0 });
        }
        let folded: Vec<char> = cmd.chars().map(|c| fold(c, self.case_sensitive)).collect();
        let latest = latest_starts(&self.chars, &folded)?;
        let boundary_first = score_path(&self.chars, &folded, &latest, true);
        let earliest = score_path(&self.chars, &folded, &latest, false);
        Some(Match { tier: Tier::Fuzzy, score: boundary_first.max(earliest) })
    }
}

/// For each pattern index `pi`, the largest text index where `pat[pi]` can be
/// matched while `pat[pi + 1..]` still fits after it. None if `pat` is not a
/// subsequence of `text`. One backward pass, so matching stays linear.
fn latest_starts(pat: &[char], text: &[char]) -> Option<Vec<usize>> {
    let mut latest = vec![0; pat.len()];
    let mut end = text.len();
    for (pi, &p) in pat.iter().enumerate().rev() {
        let i = text[..end].iter().rposition(|&c| c == p)?;
        latest[pi] = i;
        end = i;
    }
    Some(latest)
}

fn fold(c: char, case_sensitive: bool) -> char {
    if case_sensitive {
        c
    } else if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

fn is_boundary(text: &[char], i: usize) -> bool {
    i == 0 || matches!(text[i - 1], ' ' | '-' | '/' | '_' | '.' | ':' | '=' | '@' | ',')
}

/// Walk the pattern through `text`, never past `latest[pi]` so the rest of the
/// pattern always fits. With `prefer_boundary`, each pattern char takes the
/// next occurrence at a word start when one exists, which makes initials like
/// `dcup` land on `d`ocker `c`ompose `u`p.
fn score_path(pat: &[char], text: &[char], latest: &[usize], prefer_boundary: bool) -> i32 {
    const BOUNDARY: i32 = 10;
    const CONSECUTIVE: i32 = 6;
    const FIRST_CHAR: i32 = 8;
    let mut score = 0;
    let mut pos = 0usize;
    let mut prev: Option<usize> = None;
    for (pi, &p) in pat.iter().enumerate() {
        let window = pos..=latest[pi];
        let boundary = if prefer_boundary {
            window.clone().find(|&i| text[i] == p && is_boundary(text, i))
        } else {
            None
        };
        // `latest[pi]` itself matches `p`, so the window always yields an index.
        let i = boundary.unwrap_or_else(|| window.clone().find(|&i| text[i] == p).unwrap_or(latest[pi]));
        if is_boundary(text, i) {
            score += BOUNDARY;
        }
        if i == 0 {
            score += FIRST_CHAR;
        }
        match prev {
            Some(pr) if pr + 1 == i => score += CONSECUTIVE,
            Some(pr) => score -= ((i - pr - 1) as i32).min(10),
            None => score -= (i as i32).min(10),
        }
        prev = Some(i);
        pos = i + 1;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(input: &str, cmd: &str) -> Option<Match> {
        Pattern::new(input).matches(cmd)
    }

    #[test]
    fn prefix_matches_first() {
        assert_eq!(m("doc", "docker ps").unwrap().tier, Tier::Prefix);
        assert_eq!(m("git p", "git push origin master").unwrap().tier, Tier::Prefix);
    }

    #[test]
    fn initials_match_fuzzily() {
        let r = m("dcup", "docker compose up").unwrap();
        assert_eq!(r.tier, Tier::Fuzzy);
    }

    #[test]
    fn initials_score_higher_than_scattered_letters() {
        let initials = m("dcup", "docker compose up").unwrap().score;
        let scattered = m("dcup", "xdxcxuxp").unwrap().score;
        assert!(initials > scattered, "{initials} <= {scattered}");
    }

    #[test]
    fn non_matches_are_rejected() {
        assert_eq!(m("xyz", "docker compose up"), None);
        assert_eq!(m("pud", "docker compose up"), None);
        assert_eq!(m("", "anything"), None);
    }

    #[test]
    fn smart_case() {
        assert!(m("git", "GIT status").is_some());
        assert!(m("GIT", "git status").is_none());
        assert!(m("GIT", "GIT status").is_some());
    }

    #[test]
    fn long_commands_stay_fast() {
        let long = format!("echo aq z{}", " q".repeat(20_000));
        let started = std::time::Instant::now();
        assert!(m("qz", &long).is_some());
        assert!(started.elapsed().as_millis() < 20, "took {:?}", started.elapsed());
    }

    #[test]
    fn handles_non_ascii() {
        assert_eq!(m("echo \"xin", "echo \"xin chào\"").unwrap().tier, Tier::Prefix);
        assert!(m("xc", "echo \"xin chào\"").is_some());
    }
}
