//! Match a typed input against a command: prefix first, then word-start fuzzy.
//!
//! Fuzzy matching only accepts a command when the input splits into chunks that
//! each start a word of the command, in order (`dcup` → `d`ocker `c`ompose `up`).
//! A plain subsequence match is too loose: any long command contains the letters
//! of a short input somewhere, which floods the list with unrelated commands.

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

/// Input, pre-folded once per query.
pub struct Pattern {
    /// The whole input, for the prefix test.
    chars: Vec<char>,
    /// Non-whitespace input chars; `true` when whitespace preceded the char in
    /// the input, so it must start a new word.
    fuzzy: Vec<(char, bool)>,
    case_sensitive: bool,
}

impl Pattern {
    /// Smart case: an input containing an uppercase letter matches case-sensitively.
    pub fn new(input: &str) -> Self {
        let case_sensitive = input.chars().any(char::is_uppercase);
        let chars: Vec<char> = input.chars().map(|c| fold(c, case_sensitive)).collect();
        let mut fuzzy = Vec::with_capacity(chars.len());
        let mut after_space = false;
        for &c in &chars {
            if c.is_whitespace() {
                after_space = true;
            } else {
                fuzzy.push((c, after_space));
                after_space = false;
            }
        }
        Pattern { chars, fuzzy, case_sensitive }
    }

    pub fn is_empty(&self) -> bool {
        self.fuzzy.is_empty()
    }

    pub fn matches(&self, cmd: &str) -> Option<Match> {
        if self.fuzzy.is_empty() {
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
        // Linear subsequence check rejects most commands before the scoring pass.
        if !is_subsequence(&self.fuzzy, &folded) {
            return None;
        }
        let score = word_start_score(&self.fuzzy, &folded)?;
        Some(Match { tier: Tier::Fuzzy, score })
    }
}

fn is_subsequence(pat: &[(char, bool)], text: &[char]) -> bool {
    let mut it = text.iter();
    pat.iter().all(|&(p, _)| it.any(|&c| c == p))
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

fn is_separator(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\n' | '-' | '/' | '_' | '.' | ':' | '=' | '@' | ',' | '"' | '\'' | '('
            | '[' | '{' | '$' | '|' | ';' | '&' | '<' | '>'
    )
}

fn is_boundary(text: &[char], i: usize) -> bool {
    i == 0 || is_separator(text[i - 1])
}

/// Keyword search: every keyword must occur somewhere in the command, in any
/// order. Keywords are separated by commas or whitespace.
pub struct KeywordPattern {
    keywords: Vec<Vec<char>>,
    case_sensitive: bool,
}

const WHOLE_WORD: i32 = 3;
const WORD_START: i32 = 2;
const MID_WORD: i32 = 1;

impl KeywordPattern {
    /// Smart case, as in `Pattern`: any uppercase letter makes the search case-sensitive.
    pub fn new(input: &str) -> Self {
        let case_sensitive = input.chars().any(char::is_uppercase);
        let keywords = input
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|k| !k.is_empty())
            .map(|k| k.chars().map(|c| fold(c, case_sensitive)).collect())
            .collect();
        KeywordPattern { keywords, case_sensitive }
    }

    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }

    /// Score is the sum over keywords of their best occurrence: a whole word
    /// beats a word start, which beats a match inside a word. There is a single
    /// tier in keyword search, so every match uses `Tier::Prefix`.
    pub fn matches(&self, cmd: &str) -> Option<Match> {
        if self.keywords.is_empty() {
            return None;
        }
        let text: Vec<char> = cmd.chars().map(|c| fold(c, self.case_sensitive)).collect();
        let mut score = 0;
        for kw in &self.keywords {
            score += best_occurrence(kw, &text)?;
        }
        Some(Match { tier: Tier::Prefix, score })
    }
}

fn best_occurrence(kw: &[char], text: &[char]) -> Option<i32> {
    if kw.len() > text.len() {
        return None;
    }
    let mut best = None;
    for i in 0..=text.len() - kw.len() {
        if text[i..i + kw.len()] != *kw {
            continue;
        }
        let end = i + kw.len();
        let s = match (is_boundary(text, i), end == text.len() || is_separator(text[end])) {
            (true, true) => return Some(WHOLE_WORD),
            (true, false) => WORD_START,
            _ => MID_WORD,
        };
        best = best.max(Some(s));
    }
    best
}

const CONSECUTIVE: i32 = 10;
const FIRST_CHAR: i32 = 8;
const NONE: i32 = i32::MIN;

/// Best score for placing the pattern so each char either directly follows the
/// previous matched char or starts a word; a char typed after whitespace must
/// start a word. Rewards consecutive chars and penalises skipped words.
/// One row per pattern char, so the cost is O(pattern × text).
fn word_start_score(pat: &[(char, bool)], text: &[char]) -> Option<i32> {
    let n = text.len();
    // words_before[j] = word starts in text[..j].
    let mut words_before = vec![0i32; n + 1];
    for j in 0..n {
        words_before[j + 1] = words_before[j] + is_boundary(text, j) as i32;
    }
    let mut prev = vec![NONE; n];
    let mut cur = vec![NONE; n];
    for (pi, &(p, must_start)) in pat.iter().enumerate() {
        // Max over k < j of prev[k] + words_before[k + 1], so a jump from k to j
        // costs the number of word starts strictly between them.
        let mut best_before = NONE;
        for j in 0..n {
            if j > 0 && prev[j - 1] != NONE {
                best_before = best_before.max(prev[j - 1] + words_before[j]);
            }
            cur[j] = NONE;
            if text[j] != p {
                continue;
            }
            let start = is_boundary(text, j);
            if pi == 0 {
                if start {
                    cur[j] = if j == 0 { FIRST_CHAR } else { -words_before[j] };
                }
                continue;
            }
            let mut s = NONE;
            if !must_start && j > 0 && prev[j - 1] != NONE {
                s = prev[j - 1] + CONSECUTIVE;
            }
            if start && best_before != NONE {
                s = s.max(best_before - words_before[j]);
            }
            cur[j] = s;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev.into_iter().filter(|&s| s != NONE).max()
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
        assert!(m("gpom", "git push origin master").is_some());
        assert!(m("compose", "docker compose up").is_some());
        assert!(m("gi st", "git status").is_some());
    }

    #[test]
    fn letters_inside_words_do_not_match() {
        assert_eq!(m("dcup", "xdxcxuxp"), None);
        // Every letter of "git status" appears in order, but never at word starts.
        let long = "touch google-sa-dev.json user-oauth-token.json client_secret_123.json \
                    eval/runs/baseline-single-pass-translate1 eval/runs/baseline-contextual";
        assert!(is_subsequence(&Pattern::new("git status").fuzzy, &long.chars().collect::<Vec<_>>()));
        assert_eq!(m("git status", long), None);
    }

    #[test]
    fn space_in_input_starts_a_new_word() {
        assert!(m("do up", "docker compose up").is_some());
        assert_eq!(m("dock erc", "docker compose up"), None);
    }

    #[test]
    fn fewer_skipped_words_score_higher() {
        let close = m("dc", "docker compose up").unwrap().score;
        let far = m("dc", "docker run --rm alpine compose").unwrap().score;
        assert!(close > far, "{close} <= {far}");
        let whole_word = m("push", "git push origin").unwrap().score;
        let initials = m("push", "p u s h").unwrap().score;
        assert!(whole_word > initials, "{whole_word} <= {initials}");
    }

    #[test]
    fn non_matches_are_rejected() {
        assert_eq!(m("xyz", "docker compose up"), None);
        assert_eq!(m("pud", "docker compose up"), None);
        assert_eq!(m("", "anything"), None);
        assert_eq!(m("  ", "anything"), None);
    }

    #[test]
    fn smart_case() {
        assert!(m("git", "GIT status").is_some());
        assert!(m("GIT", "git status").is_none());
        assert!(m("GIT", "GIT status").is_some());
    }

    #[test]
    fn long_commands_stay_fast() {
        // Every word starts with `q`, so every position is a candidate.
        let long = format!("echo {}qz", "q ".repeat(20_000));
        let started = std::time::Instant::now();
        assert!(m("qqqz", &long).is_some());
        assert!(m("qqqy", &long).is_none());
        assert!(started.elapsed().as_millis() < 20, "took {:?}", started.elapsed());
    }

    fn k(input: &str, cmd: &str) -> Option<i32> {
        KeywordPattern::new(input).matches(cmd).map(|m| m.score)
    }

    #[test]
    fn keywords_match_in_any_order() {
        let cmd = "git pull origin master";
        assert!(k("pull", cmd).is_some());
        assert!(k("pull,master", cmd).is_some());
        assert!(k("master pull", cmd).is_some());
        assert!(k("master, pull", cmd).is_some());
        assert_eq!(k("pull,main", cmd), None);
        assert!(k("8080", "docker run -p8080:80 nginx").is_some());
    }

    #[test]
    fn empty_keywords_match_nothing() {
        assert!(KeywordPattern::new(" , ,").is_empty());
        assert_eq!(k(",", "anything"), None);
        assert_eq!(k("", "anything"), None);
    }

    #[test]
    fn whole_word_beats_word_start_beats_mid_word() {
        let whole = k("pull", "git pull").unwrap();
        let start = k("pull", "git pullrequest").unwrap();
        let mid = k("ull", "git pull").unwrap();
        assert!(whole > start && start > mid, "{whole} {start} {mid}");
        // The best occurrence counts, not the first.
        assert_eq!(k("pull", "xpull pull"), Some(whole));
        // A separator after the keyword still ends the word.
        assert_eq!(k("pull", "gh pull-request"), Some(whole));
    }

    #[test]
    fn keywords_smart_case() {
        assert!(k("pull", "GIT PULL").is_some());
        assert_eq!(k("Pull", "git pull"), None);
        assert!(k("Pull", "git Pull").is_some());
    }

    #[test]
    fn keywords_stay_fast_on_long_commands() {
        let long = format!("echo {}z", "q".repeat(40_000));
        let started = std::time::Instant::now();
        assert_eq!(k("qqqqqqqqqy", &long), None);
        assert!(started.elapsed().as_millis() < 50, "took {:?}", started.elapsed());
    }

    #[test]
    fn handles_non_ascii() {
        assert_eq!(m("echo \"xin", "echo \"xin chào\"").unwrap().tier, Tier::Prefix);
        assert!(m("xc", "echo \"xin chào\"").is_some());
    }
}
