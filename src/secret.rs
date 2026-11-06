//! Spot secrets (tokens, passwords, private keys) in a command line. Only
//! characters are scanned, no regex, so it is cheap enough to run on every
//! recorded command and on the few suggestions that are about to be shown.
//!
//! This is a safety net, not a guarantee: it looks for the common shapes of a
//! secret and errs towards not flagging plain commands.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `GITHUB_TOKEN=…`, `DB_PASSWORD=…`
    Variable,
    /// `--password x`, `mysql -px`, `sshpass -p x`
    PasswordFlag,
    /// `scheme://user:pass@host`
    UrlPassword,
    /// `curl -u user:pass`
    UserPassword,
    /// `Authorization: Bearer …`, `X-Api-Key: …`
    Header,
    /// A token whose public prefix tells what it is (`ghp_…`, `AKIA…`).
    Token { label: &'static str, prefix: &'static str },
    Jwt,
    PrivateKey,
}

impl Kind {
    /// Reads after "a": "the command looks like it contains a <label>".
    pub fn label(self) -> &'static str {
        match self {
            Kind::Variable => "secret variable",
            Kind::PasswordFlag => "password flag",
            Kind::UrlPassword => "password in a URL",
            Kind::UserPassword => "user:password pair",
            Kind::Header => "secret header",
            Kind::Token { label, .. } => label,
            Kind::Jwt => "JWT",
            Kind::PrivateKey => "private key",
        }
    }

    /// The part of the secret that is the same for every token of this kind,
    /// safe to show. `None` when every character of the value is secret.
    pub fn public_prefix(self) -> Option<&'static str> {
        match self {
            Kind::Token { prefix, .. } => Some(prefix),
            _ => None,
        }
    }
}

/// A secret value found in a command. `span` is the byte range of the value
/// alone (not the flag or variable name), always on char boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: Kind,
    pub span: Range<usize>,
}

/// `WIB_SECRET_GUARD=0` turns off hiding secrets from the log and suggestions.
pub fn guard_enabled() -> bool {
    std::env::var_os("WIB_SECRET_GUARD").is_none_or(|v| v != "0")
}

/// The first secret in `cmd`, if any.
pub fn detect(cmd: &str) -> Option<Finding> {
    detect_all(cmd).into_iter().next()
}

/// Every secret in `cmd`, in order, without overlaps.
pub fn detect_all(cmd: &str) -> Vec<Finding> {
    let b = cmd.as_bytes();
    // Every rule needs one of these characters, except the bare AWS / Google keys.
    if !b.iter().any(|c| matches!(c, b'=' | b'_' | b'-' | b':' | b'@' | b'.'))
        && !cmd.contains("AKIA")
        && !cmd.contains("AIza")
    {
        return Vec::new();
    }
    // Most specific first: when two rules see the same value, the first wins,
    // so `GITHUB_TOKEN=ghp_…` is a GitHub token (whose prefix may be shown).
    let mut found = Vec::new();
    private_keys(cmd, &mut found);
    prefixed_tokens(b, &mut found);
    headers(cmd, &mut found);
    url_passwords(b, &mut found);
    word_rules(cmd, &mut found);

    let mut kept: Vec<Finding> = Vec::new();
    for f in found {
        if !f.span.is_empty() && !kept.iter().any(|k| k.span.start < f.span.end && f.span.start < k.span.end) {
            kept.push(f);
        }
    }
    kept.sort_by_key(|f| f.span.start);
    kept
}

fn is_token_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

/// Does a token start at `i`, rather than in the middle of a word like `task-sk-…`?
fn starts_word(b: &[u8], i: usize) -> bool {
    i == 0 || !(is_token_byte(b[i - 1]) || b[i - 1] == b'.')
}

fn token_len(b: &[u8], from: usize) -> usize {
    b[from..].iter().take_while(|&&c| is_token_byte(c)).count()
}

/// (prefix, minimum length after it, label)
const PREFIXED: &[(&str, usize, &str)] = &[
    ("github_pat_", 16, "GitHub token"),
    ("ghp_", 16, "GitHub token"),
    ("gho_", 16, "GitHub token"),
    ("ghu_", 16, "GitHub token"),
    ("ghs_", 16, "GitHub token"),
    ("ghr_", 16, "GitHub token"),
    ("glpat-", 16, "GitLab token"),
    ("xoxb-", 10, "Slack token"),
    ("xoxp-", 10, "Slack token"),
    ("sk_live_", 16, "Stripe key"),
    ("rk_live_", 16, "Stripe key"),
    ("sk-", 20, "secret key"),
];

fn prefixed_tokens(b: &[u8], found: &mut Vec<Finding>) {
    for i in 0..b.len() {
        if !starts_word(b, i) {
            continue;
        }
        let rest = &b[i..];
        let token = |prefix: &'static str, label: &'static str| Kind::Token { label, prefix };
        if let Some(&(prefix, min, label)) = PREFIXED.iter().find(|(p, _, _)| rest.starts_with(p.as_bytes())) {
            let len = token_len(b, i + prefix.len());
            // `sk-` alone also starts branch names (`sk-add-payment-provider`); keys have digits.
            let looks_random = prefix != "sk-" || b[i + 3..i + 3 + len].iter().any(u8::is_ascii_digit);
            if len >= min && looks_random {
                found.push(Finding { kind: token(prefix, label), span: i..i + prefix.len() + len });
            }
        } else if rest.starts_with(b"AKIA") {
            let len = rest[4..].iter().take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit()).count();
            if len == 16 && !rest.get(20).is_some_and(|c| c.is_ascii_alphanumeric()) {
                found.push(Finding { kind: token("AKIA", "AWS access key"), span: i..i + 20 });
            }
        } else if rest.starts_with(b"AIza") {
            if token_len(b, i + 4) == 35 {
                found.push(Finding { kind: token("AIza", "Google API key"), span: i..i + 39 });
            }
        } else if rest.starts_with(b"eyJ") {
            if let Some(len) = jwt_len(rest) {
                found.push(Finding { kind: Kind::Jwt, span: i..i + len });
            }
        }
    }
}

/// `eyJ<header>.eyJ<payload>.<signature>`: two base64url JSON objects.
fn jwt_len(rest: &[u8]) -> Option<usize> {
    let b64 = |s: &[u8]| s.iter().take_while(|&&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-').count();
    let head = b64(rest);
    let payload = rest.get(head + 1..).filter(|p| head >= 10 && rest[head] == b'.' && p.starts_with(b"eyJ"))?;
    let body = b64(payload);
    if body < 10 || payload.get(body) != Some(&b'.') {
        return None;
    }
    let sig = b64(&payload[body + 1..]);
    Some(head + 1 + body + 1 + sig)
}

fn private_keys(cmd: &str, found: &mut Vec<Finding>) {
    let mut from = 0;
    while let Some(at) = cmd[from..].find("-----BEGIN ").map(|i| from + i) {
        let line_end = cmd[at..].find('\n').map_or(cmd.len(), |i| at + i);
        let Some(head_end) = cmd[at..line_end].find("PRIVATE KEY-----").map(|i| at + i + "PRIVATE KEY-----".len()) else {
            from = at + 1;
            continue;
        };
        let end = cmd[head_end..]
            .find("-----END ")
            .and_then(|i| cmd[head_end + i..].find("KEY-----").map(|j| head_end + i + j + "KEY-----".len()));
        // A header alone (`grep -- "-----BEGIN RSA PRIVATE KEY-----"`) is not a key.
        let material = cmd[head_end..end.unwrap_or(cmd.len())]
            .bytes()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'='))
            .count();
        let Some(end) = end.or((material >= 40).then_some(cmd.len())) else {
            from = head_end;
            continue;
        };
        found.push(Finding { kind: Kind::PrivateKey, span: at..end });
        from = end;
    }
}

const HEADERS: &[&str] = &["authorization:", "proxy-authorization:", "x-api-key:", "x-auth-token:", "private-token:"];
const AUTH_SCHEMES: &[&str] = &["bearer", "basic", "token", "digest", "bot", "apikey", "negotiate"];

/// The value after a header name, skipping an auth scheme like `Bearer`.
fn headers(cmd: &str, found: &mut Vec<Finding>) {
    let lower = cmd.to_ascii_lowercase();
    let b = lower.as_bytes();
    let word_end = |from: usize| from + b[from..].iter().take_while(|&&c| !c.is_ascii_whitespace() && c != b'\'' && c != b'"').count();
    let skip_blank = |from: usize| from + b[from..].iter().take_while(|&&c| c == b' ' || c == b'\t').count();
    for name in HEADERS {
        let mut from = 0;
        while let Some(at) = lower[from..].find(name).map(|i| from + i) {
            from = at + name.len();
            if at > 0 && (b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'-') {
                continue;
            }
            let mut start = skip_blank(from);
            let mut end = word_end(start);
            if AUTH_SCHEMES.contains(&&lower[start..end]) && b.get(end).is_some_and(|&c| c == b' ' || c == b'\t') {
                start = skip_blank(end);
                end = word_end(start);
            }
            if end > start && b[start] != b'$' {
                found.push(Finding { kind: Kind::Header, span: start..end });
            }
        }
    }
}

/// `scheme://user:pass@host`: the password between the first `:` and the `@`.
fn url_passwords(b: &[u8], found: &mut Vec<Finding>) {
    let mut from = 0;
    while let Some(at) = b[from..].windows(3).position(|w| w == b"://").map(|i| from + i) {
        let auth = at + 3;
        from = auth;
        let len = b[auth..]
            .iter()
            .take_while(|&&c| !c.is_ascii_whitespace() && !matches!(c, b'/' | b'?' | b'#' | b'\'' | b'"' | b'`' | b')'))
            .count();
        let authority = &b[auth..auth + len];
        let Some(at_sign) = authority.iter().rposition(|&c| c == b'@') else { continue };
        let Some(colon) = authority[..at_sign].iter().position(|&c| c == b':') else { continue };
        let (start, end) = (auth + colon + 1, auth + at_sign);
        if end > start && b[start] != b'$' {
            found.push(Finding { kind: Kind::UrlPassword, span: start..end });
        }
    }
}

/// A shell word (`op`: a lone `|`, `&`, `;`, `(` or `)`), as a byte range.
struct Word {
    span: Range<usize>,
    op: bool,
}

/// Split like the shell, close enough for spotting flags: quotes and
/// backslashes keep a word together.
fn words(cmd: &str) -> Vec<Word> {
    let b = cmd.as_bytes();
    let is_op = |c: u8| matches!(c, b'|' | b'&' | b';' | b'(' | b')');
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if is_op(c) {
            out.push(Word { span: i..i + 1, op: true });
            i += 1;
        } else {
            let start = i;
            while i < b.len() && !b[i].is_ascii_whitespace() && !is_op(b[i]) {
                match b[i] {
                    b'\'' => i += 1 + b[i + 1..].iter().position(|&c| c == b'\'').unwrap_or(b.len() - i - 1),
                    b'"' => {
                        i += 1;
                        while i < b.len() && b[i] != b'"' {
                            i += if b[i] == b'\\' { 2 } else { 1 };
                        }
                    }
                    b'\\' => i += 1,
                    _ => {}
                }
                i += 1;
            }
            // An escape at the very end can step past it.
            out.push(Word { span: start..i.min(b.len()), op: false });
            i = i.min(b.len());
        }
    }
    out
}

/// The value inside matching enclosing quotes, else the whole range.
fn unquote(cmd: &str, span: Range<usize>) -> Range<usize> {
    let s = &cmd.as_bytes()[span.clone()];
    match s {
        [q @ (b'\'' | b'"'), .., last] if s.len() >= 2 && last == q => span.start + 1..span.end - 1,
        _ => span,
    }
}

/// A value that is not a literal secret: a variable, a path, a boolean, a port.
fn not_a_secret(raw: &str) -> bool {
    let inner = raw.trim_matches(|c| c == '"' || c == '\'');
    inner.is_empty()
        || ["$", "\"$", "'$", "~/", "/"].iter().any(|p| raw.starts_with(p))
        || inner == "true"
        || inner == "false"
        || (inner.len() <= 5 && inner.bytes().all(|c| c.is_ascii_digit()))
}

/// Variable names holding a secret: a `_`-separated part is one of these…
const SECRET_PARTS: &[&str] = &["TOKEN", "SECRET", "PASSWORD", "PASSWD", "APIKEY", "CREDENTIALS"];
/// …or two adjacent parts are one of these.
const SECRET_PAIRS: &[(&str, &str)] = &[("API", "KEY"), ("ACCESS", "KEY"), ("PRIVATE", "KEY")];

/// Whole names that tools read a password from.
const SECRET_NAMES: &[&str] = &["PGPASSWORD", "MYSQL_PWD", "SSHPASS", "REDISCLI_AUTH"];

fn secret_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let parts: Vec<&str> = upper.split('_').collect();
    SECRET_NAMES.contains(&upper.as_str())
        // A part like `DBPASSWORD` or `NPMTOKEN`, but not `TOKENIZERS`.
        || parts.iter().any(|p| SECRET_PARTS.iter().any(|s| p.ends_with(s)))
        || parts.windows(2).any(|w| SECRET_PAIRS.contains(&(w[0], w[1])))
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

const LONG_FLAGS: &[&str] = &[
    "--password",
    "--passwd",
    "--token",
    "--secret",
    "--api-key",
    "--apikey",
    "--client-secret",
    "--access-token",
    "--auth-token",
    "--http-password",
    "--proxy-password",
];
const WRAPPERS: &[&str] = &["sudo", "doas", "env", "command", "exec", "time", "nohup"];
const MYSQL: &[&str] = &["mysql", "mysqldump", "mysqladmin", "mysqlimport", "mariadb", "mariadb-dump"];
const REGISTRY_CLIS: &[&str] = &["docker", "podman", "nerdctl", "buildah", "skopeo", "helm"];

/// The flag and variable rules on `cmd`, and again inside every quoted
/// word: `docker run -e "API_KEY=…"`, `bash -c 'DB_PASSWORD=… ./migrate'`.
fn word_rules(cmd: &str, found: &mut Vec<Finding>) {
    word_rules_at(cmd, 0..cmd.len(), found, 0);
}

fn word_rules_at(full: &str, range: Range<usize>, found: &mut Vec<Finding>, depth: u8) {
    let offset = range.start;
    let cmd = &full[range];
    let mut local = Vec::new();
    word_rules_in(cmd, &mut local);
    found.extend(local.into_iter().map(|f| Finding { kind: f.kind, span: f.span.start + offset..f.span.end + offset }));
    if depth >= 2 {
        return;
    }
    for w in words(cmd).iter().filter(|w| !w.op) {
        let b = cmd.as_bytes();
        // Every quoted run inside the word, e.g. both halves of `-e"A=1"'B=2'`.
        let mut i = w.span.start;
        while i < w.span.end {
            match b[i] {
                q @ (b'\'' | b'"') => {
                    let close = b[i + 1..w.span.end].iter().position(|&c| c == q).map_or(w.span.end, |p| i + 1 + p);
                    if close > i + 1 {
                        word_rules_at(full, offset + i + 1..offset + close, found, depth + 1);
                    }
                    i = close + 1;
                }
                b'\\' => i += 2,
                _ => i += 1,
            }
        }
    }
}

fn word_rules_in(cmd: &str, found: &mut Vec<Finding>) {
    let all = words(cmd);
    for segment in all.split(|w| w.op) {
        let text = |w: &Word| &cmd[w.span.clone()];
        let push = |found: &mut Vec<Finding>, kind: Kind, span: Range<usize>| {
            if !not_a_secret(&cmd[span.clone()]) {
                found.push(Finding { kind, span: unquote(cmd, span) });
            }
        };

        // The program: the first word that is not an assignment or a wrapper.
        let mut wrapped = false;
        let prog_at = segment.iter().position(|w| {
            let t = text(w);
            let assignment = t.split_once('=').is_some_and(|(name, _)| is_identifier(name));
            let skip = assignment || WRAPPERS.contains(&t) || (wrapped && t.starts_with('-'));
            wrapped |= WRAPPERS.contains(&t);
            !skip
        });
        let prog = prog_at.map_or("", |i| text(&segment[i]).rsplit('/').next().unwrap_or(""));
        let login = segment.iter().any(|w| text(w) == "login");

        for (k, w) in segment.iter().enumerate() {
            let t = text(w);
            let next = segment.get(k + 1).map(|n| n.span.clone());
            let after = |n: usize| w.span.start + n..w.span.end;

            if let Some((name, _)) = t.split_once('=') {
                if is_identifier(name) && secret_name(name) {
                    push(found, Kind::Variable, after(name.len() + 1));
                    continue;
                }
            }
            if let Some(flag) = LONG_FLAGS.iter().find(|f| t.starts_with(**f)) {
                match &t[flag.len()..] {
                    "" => {
                        if let Some(n) = next.clone() {
                            push(found, Kind::PasswordFlag, n);
                        }
                    }
                    rest if rest.starts_with('=') => push(found, Kind::PasswordFlag, after(flag.len() + 1)),
                    _ => {}
                }
                continue;
            }
            let short_p = t == "-p" && (prog == "sshpass" || (login && REGISTRY_CLIS.contains(&prog)));
            if short_p {
                if let Some(n) = next {
                    push(found, Kind::PasswordFlag, n);
                }
            } else if t.len() > 2 && t.starts_with("-p") && (MYSQL.contains(&prog) || prog == "sshpass") {
                push(found, Kind::PasswordFlag, after(2));
            } else if prog == "curl" {
                let value = match t {
                    "-u" | "--user" => next,
                    _ if t.starts_with("--user=") => Some(after(7)),
                    _ if t.starts_with("-u") && t.len() > 2 && !t.starts_with("--") => Some(after(2)),
                    _ => None,
                };
                if let Some(v) = value.filter(|v| !not_a_secret(&cmd[v.clone()])) {
                    let inner = unquote(cmd, v);
                    if let Some(colon) = cmd[inner.clone()].find(':') {
                        let pass = inner.start + colon + 1..inner.end;
                        if !pass.is_empty() && !cmd[pass.clone()].starts_with('$') {
                            found.push(Finding { kind: Kind::UserPassword, span: pass });
                        }
                    }
                }
            }
        }

        // `htpasswd -b [-c] file user password` / `htpasswd -nb user password`.
        if prog == "htpasswd" {
            let rest = &segment[prog_at.map_or(0, |i| i + 1)..];
            let batch = rest.iter().any(|w| {
                let t = text(w);
                t.starts_with('-') && !t.starts_with("--") && t.contains('b')
            });
            let positional: Vec<&Word> = rest.iter().filter(|w| !text(w).starts_with('-')).collect();
            if let (true, [_, _, ..]) = (batch, positional.as_slice()) {
                push(found, Kind::PasswordFlag, positional[positional.len() - 1].span.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The secret values found in `cmd`, as text.
    fn values(cmd: &str) -> Vec<&str> {
        detect_all(cmd).into_iter().map(|f| &cmd[f.span]).collect()
    }

    fn kinds(cmd: &str) -> Vec<Kind> {
        detect_all(cmd).into_iter().map(|f| f.kind).collect()
    }

    #[test]
    fn variables() {
        assert_eq!(values("export GITHUB_TOKEN=abc123def456"), ["abc123def456"]);
        assert_eq!(values("DB_PASSWORD='p@ss w0rd' ./run"), ["p@ss w0rd"]);
        assert_eq!(values("AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI ./x"), ["wJalrXUtnFEMI"]);
        assert_eq!(values("STRIPE_API_KEY=abcdef"), ["abcdef"]);
        assert_eq!(values("db_passwd=hunter2 psql"), ["hunter2"]);
        assert_eq!(kinds("MY_APIKEY=zzz"), [Kind::Variable]);
        assert_eq!(values("docker run -e PRIVATE_KEY=abc img"), ["abc"]);
    }

    #[test]
    fn variables_that_are_not_secrets() {
        for cmd in [
            "TOKENIZERS_PARALLELISM=false python x.py",
            "PASSWORD_STORE_DIR=~/x pass ls",
            "TOKEN=\"$(op read op://vault/item)\" ./run",
            "TOKEN=$GITHUB_TOKEN ./run",
            "SECRET_FILE=/run/s ./run",
            "TOKEN= ./run",
            "TOKEN='' ./run",
            "API_TOKEN=true make",
            "SECRET_PORT=8080 make",
            "KEYBOARD=us setxkbmap",
        ] {
            assert!(detect_all(cmd).is_empty(), "{cmd}");
        }
    }

    #[test]
    fn password_flags() {
        assert_eq!(values("tool --password=hunter2"), ["hunter2"]);
        assert_eq!(values("tool --password hunter2"), ["hunter2"]);
        assert_eq!(values("gh auth --token abc"), ["abc"]);
        assert_eq!(values("vault --secret abc"), ["abc"]);
        assert_eq!(values("x --api-key 'k e y'"), ["k e y"]);
        assert_eq!(values("az login --client-secret abc"), ["abc"]);
        assert_eq!(values("wget --password x http://h/f"), ["x"]);
        assert!(detect_all("tool --password-file x").is_empty());
        assert!(detect_all("tool --token $T").is_empty());
        assert!(detect_all("tool --token \"$T\"").is_empty());
    }

    #[test]
    fn program_short_flags() {
        assert_eq!(values("mysql -u root -pS3cret db"), ["S3cret"]);
        assert_eq!(values("sshpass -p pw ssh host"), ["pw"]);
        assert_eq!(values("docker login -u me -p pw registry.io"), ["pw"]);
        assert_eq!(values("htpasswd -b file user pw"), ["pw"]);
        assert_eq!(values("htpasswd -nb user pw"), ["pw"]);
        assert_eq!(values("sudo mysql -pS3cret"), ["S3cret"]);
        assert_eq!(values("cd x && mysql -pS3cret"), ["S3cret"]);
        for cmd in ["mysql -p", "mysql -u root -p db", "docker run -p 8080:80 nginx", "ssh -p 22 host", "htpasswd file user"] {
            assert!(detect_all(cmd).is_empty(), "{cmd}");
        }
    }

    #[test]
    fn user_and_password() {
        assert_eq!(values("curl -u user:pass https://x"), ["pass"]);
        assert_eq!(values("curl --user user:pass https://x"), ["pass"]);
        assert_eq!(values("curl -uuser:pass https://x"), ["pass"]);
        assert_eq!(values("curl --user='me:p w' https://x"), ["p w"]);
        assert_eq!(values("psql postgres://app:s3cret@db.local/app"), ["s3cret"]);
        assert_eq!(kinds("git clone https://me:tok@host/r"), [Kind::UrlPassword]);
        for cmd in [
            "curl -u user https://x",
            "curl https://example.com/@user",
            "git clone git@github.com:org/repo",
            "curl -u \"$U:$P\" x",
            "docker run --user 1000:1000 img",
            "open https://me@host/x",
        ] {
            assert!(detect_all(cmd).is_empty(), "{cmd}");
        }
    }

    #[test]
    fn headers_any_case() {
        assert_eq!(values("curl -H 'Authorization: Bearer abc.def' x"), ["abc.def"]);
        assert_eq!(values("curl -H \"authorization: basic dXNlcjpwYXNz\" x"), ["dXNlcjpwYXNz"]);
        assert_eq!(values("curl -H 'X-Api-Key: k123' x"), ["k123"]);
        assert_eq!(values("curl -H 'Authorization: rawtoken' x"), ["rawtoken"]);
        assert!(detect_all("curl -H 'Authorization: $AUTH' x").is_empty());
        assert!(detect_all("curl -H \"Authorization: Bearer $TOKEN\" x").is_empty());
    }

    #[test]
    fn prefixed_tokens_at_word_start() {
        let ghp = "ghp_0123456789abcdefghijABCDEFGHIJ012345";
        assert_eq!(values(&format!("echo {ghp}")), [ghp]);
        assert_eq!(detect(&format!("echo {ghp}")).unwrap().kind.public_prefix(), Some("ghp_"));
        let akia = "AKIAIOSFODNN7EXAMPLE";
        assert_eq!(values(&format!("aws configure set key {akia}")), [akia]);
        let aiza = format!("AIza{}", "a".repeat(35));
        assert_eq!(values(&format!("curl x?key={aiza}")), [aiza.as_str()]);
        assert_eq!(values("x sk-abcdefghij0123456789"), ["sk-abcdefghij0123456789"]);
        assert_eq!(values("x glpat-abcdefghij0123456789"), ["glpat-abcdefghij0123456789"]);
        assert_eq!(values("x xoxb-1234567890-abc"), ["xoxb-1234567890-abc"]);
        assert_eq!(values("x sk_live_abcdefghij0123456789"), ["sk_live_abcdefghij0123456789"]);
        for cmd in ["git checkout task-sk-1234567890abcdefghij", "echo AKIA1234", "echo ghp_short", "echo AKIAIOSFODNN7EXAMPLEX"] {
            assert!(detect_all(cmd).is_empty(), "{cmd}");
        }
    }

    #[test]
    fn variable_holding_a_known_token_keeps_its_prefix() {
        let cmd = "export GITHUB_TOKEN=ghp_0123456789abcdefghijABCDEF";
        let f = detect_all(cmd);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind.label(), "GitHub token");
        assert_eq!(&cmd[f[0].span.clone()], "ghp_0123456789abcdefghijABCDEF");
    }

    #[test]
    fn jwt_and_private_key() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert_eq!(values(&format!("curl -d token={jwt} x")), [jwt]);
        let key = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaA\n-----END OPENSSH PRIVATE KEY-----";
        let cmd = format!("echo '{key}' > id");
        assert_eq!(values(&cmd), [key]);
        assert_eq!(kinds(&cmd), [Kind::PrivateKey]);
        assert!(detect_all("echo '-----BEGIN CERTIFICATE-----'").is_empty());
    }

    #[test]
    fn two_secrets_in_one_command() {
        let cmd = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI aws s3 ls";
        assert_eq!(values(cmd), ["AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI"]);
    }

    #[test]
    fn multibyte_text_around_secrets() {
        assert_eq!(values("echo «xin chào» && API_TOKEN=bí-mật✓ ./chạy"), ["bí-mật✓"]);
        assert_eq!(values("curl -u tên:mật-khẩu https://x/đường"), ["mật-khẩu"]);
        assert!(detect_all("echo «xin chào» ✓ — đường/dẫn").is_empty());
        // Unterminated quotes and a trailing backslash do not panic.
        for cmd in ["echo 'abc", "echo \"abc\\", "echo \\", "TOKEN='abc", "x --password"] {
            let _ = detect_all(cmd);
        }
    }

    #[test]
    fn assignments_inside_quotes() {
        assert_eq!(values("docker run -e \"API_KEY=abc123\" img"), ["abc123"]);
        assert_eq!(values("docker build --build-arg 'NPM_TOKEN=abc123' ."), ["abc123"]);
        assert_eq!(values("bash -c 'DB_PASSWORD=hunter2 ./migrate'"), ["hunter2"]);
        assert_eq!(values("ssh prod \"export API_TOKEN=abc123xyz\""), ["abc123xyz"]);
        assert_eq!(values("export \"GITHUB_TOKEN=abc123\""), ["abc123"]);
        assert_eq!(values("sh -c \"mysql -pS3cret db\""), ["S3cret"]);
        assert!(detect_all("docker run -e \"TOKEN=$T\" img").is_empty());
        assert!(detect_all("echo 'TOKENIZERS_PARALLELISM=false'").is_empty());
    }

    #[test]
    fn password_variables_tools_read() {
        assert_eq!(values("PGPASSWORD=hunter2 psql"), ["hunter2"]);
        assert_eq!(values("MYSQL_PWD=hunter2 mysql"), ["hunter2"]);
        assert_eq!(values("SSHPASS=hunter2 sshpass -e ssh h"), ["hunter2"]);
        assert_eq!(values("NPMTOKEN=abc123 npm publish"), ["abc123"]);
        assert!(detect_all("OLDPWD=/x cd -").is_empty());
    }

    #[test]
    fn lookalikes_are_not_secrets() {
        for cmd in [
            "git checkout sk-add-new-payment-provider",
            "cd sk-hynix-memory-benchmarks",
            "grep -r \"-----BEGIN RSA PRIVATE KEY-----\" .",
            "echo 'api-key: see the docs'",
        ] {
            assert!(detect_all(cmd).is_empty(), "{cmd}");
        }
        let key = format!("-----BEGIN RSA PRIVATE KEY-----\n{}", "A".repeat(64));
        assert_eq!(kinds(&format!("echo '{key}'")), [Kind::PrivateKey]);
    }

    #[test]
    fn plain_commands_have_no_secrets() {
        for cmd in ["git push origin master", "ls -la", "docker compose up -d", "kubectl get pods -A", "cargo test --release"] {
            assert!(detect(cmd).is_none(), "{cmd}");
        }
    }
}
