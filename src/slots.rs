//! Template slots: the parts of a command that earlier runs of the same
//! command changed (a port, an image tag, a pod name), so Ctrl-] can jump to
//! them. Nothing is guessed from the shape of a word: without an earlier run
//! of the same shape there are no slots.

use crate::entry::Entry;
use crate::rank::RankContext;
use std::collections::HashSet;

/// A shell word with its char range in the command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Split like the shell: words are separated by whitespace, `'…'` and `"…"`
/// keep their spaces, and `\` escapes the next character. Quotes stay in the
/// text, so `"a b"` and `'a b'` are different values.
pub fn tokenize(cmd: &str) -> Vec<Token> {
    let chars: Vec<char> = cmd.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let mut quote: Option<char> = None;
        while i < chars.len() {
            let c = chars[i];
            match quote {
                Some(q) if c == q => quote = None,
                Some('"') if c == '\\' => i += 1,
                Some(_) => {}
                None if c.is_whitespace() => break,
                None if c == '\'' || c == '"' => quote = Some(c),
                None if c == '\\' => i += 1,
                None => {}
            }
            i += 1;
        }
        let end = i.min(chars.len());
        out.push(Token { text: chars[start..end].iter().collect(), start, end });
        i = end;
    }
    out
}

/// Char ranges of the slots in `buffer`, in order. A sibling is an earlier
/// command with as many words, the same first word, and at least half of its
/// words equal to the buffer's; a slot is a word (past the first) where the
/// buffer and its siblings differ.
pub fn find(buffer: &str, entries: &[Entry], ctx: RankContext) -> Vec<(usize, usize)> {
    let words = tokenize(buffer);
    if words.len() < 2 {
        return Vec::new();
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut siblings: Vec<Vec<Token>> = Vec::new();
    for e in entries {
        let cmd = e.cmd.trim();
        // Cheap test before tokenizing: the program must be the same.
        if !cmd.starts_with(words[0].text.as_str()) || !seen.insert(cmd) || ctx.hides(cmd) {
            continue;
        }
        let other = tokenize(cmd);
        if other.len() != words.len() || other[0].text != words[0].text {
            continue;
        }
        let same = words.iter().zip(&other).filter(|(a, b)| a.text == b.text).count();
        if same * 2 >= words.len() {
            siblings.push(other);
        }
    }

    let mut slots = Vec::new();
    for (i, word) in words.iter().enumerate().skip(1) {
        let values: Vec<&str> = std::iter::once(word.text.as_str()).chain(siblings.iter().map(|s| s[i].text.as_str())).collect();
        if values.iter().all(|v| *v == values[0]) {
            continue;
        }
        // `--tag=1.2` / `--tag=1.3`: only the part after `=` changes.
        let key = |v: &str| v.split_once('=').map(|(k, _)| k.to_string());
        let start = match key(values[0]) {
            Some(k) if values.iter().all(|v| key(v).as_deref() == Some(k.as_str())) => word.start + k.chars().count() + 1,
            _ => word.start,
        };
        slots.push((start, word.end));
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(cmds: &[&str]) -> Vec<Entry> {
        cmds.iter().map(|c| Entry { ts: 0, cmd: c.to_string(), cwd: None, exit: Some(0) }).collect()
    }

    fn texts(buffer: &str, cmds: &[&str]) -> Vec<String> {
        let chars: Vec<char> = buffer.chars().collect();
        find(buffer, &runs(cmds), RankContext::none())
            .into_iter()
            .map(|(s, e)| chars[s..e].iter().collect())
            .collect()
    }

    #[test]
    fn tokenizes_like_the_shell() {
        let t: Vec<String> = tokenize(r#"echo "a b" 'c d' e\ f  g"#).into_iter().map(|t| t.text).collect();
        assert_eq!(t, [r#"echo"#, r#""a b""#, "'c d'", r"e\ f", "g"]);
        let t = tokenize("ls  -la");
        assert_eq!((t[1].start, t[1].end), (4, 7));
        // Unterminated quotes and a trailing backslash do not panic.
        for cmd in ["echo 'abc", "echo \"abc\\", "echo \\"] {
            assert_eq!(tokenize(cmd).len(), 2, "{cmd}");
        }
    }

    #[test]
    fn port_and_tag_vary() {
        let buffer = "docker run -p 8080:80 nginx:1.25";
        let siblings = ["docker run -p 9090:80 nginx:1.24", "docker run -p 3000:80 nginx:1.25"];
        assert_eq!(texts(buffer, &siblings), ["8080:80", "nginx:1.25"]);
        assert_eq!(find(buffer, &runs(&siblings), RankContext::none()), [(14, 21), (22, 32)]);
    }

    #[test]
    fn one_slot_for_a_pod_name() {
        assert_eq!(texts("kubectl logs pod-a", &["kubectl logs pod-b", "kubectl get pods"]), ["pod-a"]);
    }

    #[test]
    fn quoted_value_is_one_slot() {
        assert_eq!(texts(r#"git commit -m "fix bug""#, &[r#"git commit -m "add feature""#]), [r#""fix bug""#]);
    }

    #[test]
    fn narrows_to_the_value_after_equals() {
        assert_eq!(texts("helm install --version=1.2 app", &["helm install --version=1.3 app"]), ["1.2"]);
        // Different keys: the whole word is the slot.
        assert_eq!(texts("tool --a=1 x", &["tool --b=1 x"]), ["--a=1"]);
    }

    #[test]
    fn no_sibling_no_slot() {
        assert!(texts("docker run -p 8080:80 nginx", &[]).is_empty());
        assert!(texts("docker run -p 8080:80 nginx", &["docker ps", "podman run -p 1:1 x"]).is_empty());
        // Fewer than half the words in common.
        assert!(texts("a b c d", &["a x y z"]).is_empty());
        assert!(texts("ls", &["ls -la"]).is_empty());
    }

    #[test]
    fn offsets_are_chars() {
        let buffer = "echo «xin» chào";
        assert_eq!(find(buffer, &runs(&["echo «xin» bạn"]), RankContext::none()), [(11, 15)]);
    }

    #[test]
    fn forgotten_and_secret_siblings_do_not_count() {
        let entries = runs(&["mysql -u root -pS3cretPw db", "mysql -u admin db2 x"]);
        let forgotten: HashSet<String> = ["kubectl logs pod-b".to_string()].into();
        let ctx = RankContext { forgotten: Some(&forgotten), secret_guard: true, ..RankContext::none() };
        assert!(find("kubectl logs pod-a", &runs(&["kubectl logs pod-b"]), ctx).is_empty());
        assert!(find("mysql -u root -pOther db", &entries, ctx).is_empty());
        assert_eq!(find("mysql -u root -pOther db", &entries, RankContext::none()).len(), 1);
    }
}
