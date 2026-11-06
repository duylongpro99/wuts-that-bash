//! `wib doctor`: check the installation and say how to fix what is wrong.
//! The zsh function `wib` passes the shell's own state (options, key
//! bindings, unexported settings) in `_WIB_DOCTOR_STATE` as `key=value` lines.
//! Every check is a plain function of its inputs, so it can be tested without
//! a shell.

use crate::entry::parse_period;
use crate::log_store::{self, FORGET_FILE, LOG_FILE};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    Info,
    Warn,
    Fail,
}

impl Level {
    fn word(self) -> &'static str {
        match self {
            Level::Ok => "ok",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub level: Level,
    pub what: String,
    pub fix: Option<String>,
}

fn ok(what: impl Into<String>) -> Check {
    Check { level: Level::Ok, what: what.into(), fix: None }
}

fn bad(level: Level, what: impl Into<String>, fix: impl Into<String>) -> Check {
    Check { level, what: what.into(), fix: Some(fix.into()) }
}

pub type State = HashMap<String, String>;

pub fn parse_state(text: &str) -> State {
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

pub fn loaded(state: Option<&State>) -> Check {
    match state {
        Some(_) => ok("wib is loaded in this shell"),
        None => bad(
            Level::Warn,
            "not run from a shell with wib loaded, so shell settings are not checked",
            "add `eval \"$(wib init zsh)\"` at the end of ~/.zshrc, open a new terminal and run `wib doctor` there",
        ),
    }
}

/// Lines that load plugins which rebind keys or wrap the line editor.
fn loads_plugins(line: &str) -> bool {
    let sources = line.split_whitespace().next().is_some_and(|w| w == "source" || w == ".");
    line.contains("oh-my-zsh.sh")
        || line.contains("zinit")
        || line.contains("antidote load")
        || line.contains("antigen apply")
        || line.contains("zplug load")
        || (sources && (line.contains("zsh-autosuggestions") || line.contains("zsh-syntax-highlighting")))
}

/// `.zshrc` must load wib after plugins that bind the same keys.
pub fn load_order(zshrc: Option<&str>, name: &str) -> Check {
    let Some(text) = zshrc else {
        return Check { level: Level::Info, what: format!("{name} not found; load order not checked"), fix: None };
    };
    let lines: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.starts_with('#'))
        .collect();
    let Some(&(wib_at, _)) = lines.iter().find(|(_, l)| l.contains("wib init zsh")) else {
        return Check { level: Level::Info, what: format!("no `wib init zsh` line in {name}"), fix: None };
    };
    match lines.iter().find(|(n, l)| *n > wib_at && loads_plugins(l)) {
        Some((n, l)) => bad(
            Level::Warn,
            format!("{name}:{wib_at} loads wib before {name}:{n} (`{l}`), which may take wib's keys"),
            format!("move the `eval \"$(wib init zsh)\"` line below line {n}"),
        ),
        None => ok(format!("{name} loads wib after its plugins")),
    }
}

pub fn zsh_version(version: &str) -> Check {
    let mut parts = version.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let (major, minor) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    if (major, minor) >= (5, 3) {
        ok(format!("zsh {version}"))
    } else {
        bad(Level::Fail, format!("zsh {version} is older than 5.3"), "install a newer zsh (e.g. `brew install zsh`)")
    }
}

/// zsh writes timestamps only with EXTENDED_HISTORY; wib needs them.
pub fn extended_history(option: Option<&str>, tail: &[&str]) -> Check {
    let fix = "add `setopt EXTENDED_HISTORY` to ~/.zshrc";
    if option == Some("off") {
        return bad(Level::Fail, "EXTENDED_HISTORY is off, so the history has no timestamps", fix);
    }
    let stamped = |l: &&str| {
        l.strip_prefix(": ")
            .and_then(|r| r.split_once(';'))
            .and_then(|(head, _)| head.split_once(':'))
            .is_some_and(|(ts, d)| !ts.is_empty() && ts.bytes().all(|b| b.is_ascii_digit()) && d.bytes().all(|b| b.is_ascii_digit()))
    };
    if !tail.is_empty() && !tail.iter().any(stamped) {
        return bad(Level::Fail, "the last lines of the history file have no timestamps", fix);
    }
    ok("history has timestamps")
}

/// (sequence as `bindkey` prints it, name)
const KEYS: &[(&str, &str)] = &[
    ("^[[B", "Down"),
    ("^[OB", "Down"),
    ("^[[Z", "Shift+Tab"),
    ("^[", "Esc"),
    ("^G", "Ctrl-G"),
    ("^]", "Ctrl-]"),
];

pub fn key_bindings(state: &State) -> Vec<Check> {
    let taken: Vec<String> = KEYS
        .iter()
        .filter_map(|(seq, name)| {
            let widget = state.get(&format!("bind:{seq}")).map(String::as_str).unwrap_or("undefined-key");
            (!widget.starts_with("_wib_")).then(|| format!("{name} ({seq}) runs {widget}"))
        })
        .collect();
    if taken.is_empty() {
        return vec![ok("Down, Shift+Tab, Esc, Ctrl-G and Ctrl-] are bound to wib")];
    }
    taken
        .into_iter()
        .map(|t| bad(Level::Warn, format!("{t}, not wib"), "load wib after the plugin or binding that takes this key"))
        .collect()
}

/// What `stat` says about a path, so the data directory check is testable.
pub struct Meta {
    pub uid: u32,
    pub mode: u32,
    pub symlink: bool,
}

fn meta(path: &Path) -> Option<Meta> {
    let m = fs::symlink_metadata(path).ok()?;
    Some(Meta { uid: m.uid(), mode: m.mode() & 0o7777, symlink: m.file_type().is_symlink() })
}

pub fn data_dir(dir: &str, dir_meta: Option<&Meta>, files: &[(&str, Option<Meta>)], euid: u32) -> Vec<Check> {
    let Some(d) = dir_meta else {
        return vec![ok(format!("{dir} does not exist yet; wib creates it on the first command"))];
    };
    let mut out = Vec::new();
    if d.symlink {
        out.push(bad(Level::Fail, format!("{dir} is a symlink"), format!("replace it with a real directory: `rm {dir} && mkdir -m 700 {dir}`")));
    }
    if d.uid != euid && euid != 0 {
        out.push(bad(Level::Fail, format!("{dir} is owned by another user"), format!("`sudo chown -R $(id -u) {dir}`")));
    }
    if d.mode & 0o022 != 0 {
        out.push(bad(Level::Fail, format!("{dir} is writable by group or others (mode {:o})", d.mode), format!("`chmod 700 {dir}`")));
    }
    for (name, m) in files {
        let path = format!("{dir}/{name}");
        match m {
            Some(m) if m.symlink => out.push(bad(Level::Fail, format!("{path} is a symlink"), format!("`rm {path}`"))),
            Some(m) if m.mode & 0o077 != 0 => {
                out.push(bad(Level::Fail, format!("{path} is readable by others (mode {:o})", m.mode), format!("`chmod 600 {path}`")))
            }
            _ => {}
        }
    }
    if out.is_empty() {
        out.push(ok(format!("{dir} is private")));
    }
    out
}

/// `WIB_PERIOD`, `WIB_LIMIT` and `WIB_MIN_CHARS`, as the shell has them.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Vec<Check> {
    let mut out = Vec::new();
    let set = |name: &str| get(name).filter(|v| !v.is_empty());
    if let Some(v) = set("WIB_PERIOD") {
        if let Err(e) = parse_period(&v) {
            out.push(bad(Level::Fail, format!("WIB_PERIOD: {e}"), "e.g. `WIB_PERIOD=30d` (h, d or w)"));
        }
    }
    for (name, example) in [("WIB_LIMIT", "8"), ("WIB_MIN_CHARS", "2")] {
        if let Some(v) = set(name) {
            if v.parse::<usize>().is_err() {
                out.push(bad(Level::Fail, format!("{name}={v} is not a number"), format!("e.g. `{name}={example}`")));
            }
        }
    }
    if let Some(v) = set("KEYTIMEOUT").and_then(|v| v.parse::<u32>().ok()).filter(|&t| t > 10) {
        out.push(bad(Level::Info, format!("KEYTIMEOUT={v}: Esc waits {}ms before hiding the list", v * 10), "`KEYTIMEOUT=1` if Esc feels slow"));
    }
    if set("WIB_SECRET_GUARD").as_deref() == Some("0") {
        out.push(Check { level: Level::Info, what: "WIB_SECRET_GUARD=0: commands holding secrets are saved and suggested".into(), fix: None });
    }
    if out.is_empty() {
        out.push(ok("settings are valid"));
    }
    out
}

pub fn speed(ms: u128, entries: usize) -> Check {
    if ms > 50 {
        bad(Level::Warn, format!("a lookup took {ms} ms over {entries} commands"), "shorten the period, e.g. `WIB_PERIOD=14d`")
    } else {
        ok(format!("a lookup takes {ms} ms over {entries} commands"))
    }
}

pub fn secrets(found: usize) -> Check {
    if found == 0 {
        ok("no secrets in recent history")
    } else {
        bad(Level::Warn, format!("{found} recent commands look like they contain secrets"), "run `wib audit`")
    }
}

/// Run every check and print them. Returns whether any failed.
pub fn run(zsh_history: &Path, period_secs: i64, now: i64) -> bool {
    let state = std::env::var("_WIB_DOCTOR_STATE").ok().map(|s| parse_state(&s));
    let get = |name: &str| match &state {
        Some(s) => s.get(name).cloned(),
        None => std::env::var(name).ok(),
    };
    let mut checks = vec![loaded(state.as_ref())];

    let home = log_store::home();
    let zdotdir = get("ZDOTDIR").filter(|d| !d.is_empty()).map_or(home.clone(), std::path::PathBuf::from);
    let zshrc = zdotdir.join(".zshrc");
    let shown = match zshrc.strip_prefix(&home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => zshrc.display().to_string(),
    };
    checks.push(load_order(fs::read_to_string(&zshrc).ok().as_deref(), &shown));

    if let Some(s) = &state {
        checks.push(zsh_version(s.get("zsh_version").map_or("", String::as_str)));
    }
    let raw = fs::read(zsh_history).unwrap_or_default();
    let text = String::from_utf8_lossy(&raw);
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    let tail = &lines[lines.len().saturating_sub(20)..];
    checks.push(extended_history(get("extended_history").as_deref(), tail));
    if let Some(s) = &state {
        checks.extend(key_bindings(s));
    }

    let dir = log_store::data_dir();
    let files: Vec<(&str, Option<Meta>)> = [LOG_FILE, FORGET_FILE].iter().map(|f| (*f, meta(&dir.join(f)))).collect();
    checks.extend(data_dir(&dir.display().to_string(), meta(&dir).as_ref(), &files, log_store::euid()));
    checks.extend(settings(get));

    let started = Instant::now();
    let entries = crate::load_entries(period_secs, now);
    let _ = crate::rank::rank(&entries, "gi", now, 8, crate::rank::RankContext::default());
    checks.push(speed(started.elapsed().as_millis(), entries.len()));
    let mut with_secret: Vec<&str> = entries.iter().map(|e| e.cmd.trim()).filter(|c| crate::secret::detect(c).is_some()).collect();
    with_secret.sort_unstable();
    with_secret.dedup();
    checks.push(secrets(with_secret.len()));

    for c in &checks {
        println!("{:<5} {}", c.level.word(), c.what);
        if let Some(fix) = &c.fix {
            println!("      fix: {fix}");
        }
    }
    checks.iter().any(|c| c.level == Level::Fail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(checks: &[Check]) -> Vec<Level> {
        checks.iter().map(|c| c.level).collect()
    }

    #[test]
    fn load_order_wants_wib_after_plugins() {
        let before = "export ZSH=~/.oh-my-zsh\neval \"$(wib init zsh)\"\nsource $ZSH/oh-my-zsh.sh\n";
        let c = load_order(Some(before), "~/.zshrc");
        assert_eq!(c.level, Level::Warn);
        assert!(c.what.contains("~/.zshrc:2") && c.what.contains("~/.zshrc:3"), "{}", c.what);
        let after = "source $ZSH/oh-my-zsh.sh\nsource ~/p/zsh-autosuggestions/zsh-autosuggestions.zsh\neval \"$(wib init zsh)\"\n";
        assert_eq!(load_order(Some(after), "x").level, Level::Ok);
        let commented = "eval \"$(wib init zsh)\"\n# source $ZSH/oh-my-zsh.sh\n";
        assert_eq!(load_order(Some(commented), "x").level, Level::Ok);
        assert_eq!(load_order(Some("eval \"$(wib init zsh)\"\nzinit light foo\n"), "x").level, Level::Warn);
        assert_eq!(load_order(Some("ls\n"), "x").level, Level::Info);
        assert_eq!(load_order(None, "x").level, Level::Info);
    }

    #[test]
    fn zsh_versions() {
        assert_eq!(zsh_version("5.9").level, Level::Ok);
        assert_eq!(zsh_version("5.3.1").level, Level::Ok);
        assert_eq!(zsh_version("5.2").level, Level::Fail);
        assert_eq!(zsh_version("4.3.17").level, Level::Fail);
    }

    #[test]
    fn extended_history_needs_timestamps() {
        assert_eq!(extended_history(Some("on"), &[": 1700000000:0;ls"]).level, Level::Ok);
        assert_eq!(extended_history(Some("off"), &[": 1700000000:0;ls"]).level, Level::Fail);
        assert_eq!(extended_history(None, &["ls", "git status"]).level, Level::Fail);
        assert_eq!(extended_history(None, &[]).level, Level::Ok);
    }

    #[test]
    fn keys_taken_by_others_are_named() {
        let mut state: State = KEYS.iter().map(|(k, _)| (format!("bind:{k}"), "_wib_x".to_string())).collect();
        assert_eq!(levels(&key_bindings(&state)), [Level::Ok]);
        state.insert("bind:^[[B".into(), "history-substring-search-down".into());
        let checks = key_bindings(&state);
        assert_eq!(levels(&checks), [Level::Warn]);
        assert!(checks[0].what.contains("history-substring-search-down"));
    }

    #[test]
    fn data_directory_must_be_private() {
        let dir = |mode, uid| Meta { uid, mode, symlink: false };
        let file = |mode| Some(Meta { uid: 501, mode, symlink: false });
        assert_eq!(levels(&data_dir("/d", Some(&dir(0o700, 501)), &[("history.tsv", file(0o600))], 501)), [Level::Ok]);
        assert_eq!(levels(&data_dir("/d", Some(&dir(0o777, 501)), &[], 501)), [Level::Fail]);
        assert_eq!(levels(&data_dir("/d", Some(&dir(0o700, 0)), &[], 501)), [Level::Fail]);
        assert_eq!(levels(&data_dir("/d", Some(&dir(0o700, 501)), &[("forget", file(0o644))], 501)), [Level::Fail]);
        assert_eq!(levels(&data_dir("/d", None, &[], 501)), [Level::Ok]);
        let fix = data_dir("/d", Some(&dir(0o777, 501)), &[], 501)[0].fix.clone().unwrap();
        assert!(fix.contains("chmod 700 /d"));
    }

    #[test]
    fn settings_are_validated() {
        let with = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
        };
        assert_eq!(levels(&settings(with(&[]))), [Level::Ok]);
        assert_eq!(levels(&settings(with(&[("WIB_PERIOD", "7x")]))), [Level::Fail]);
        assert_eq!(levels(&settings(with(&[("WIB_LIMIT", "x"), ("WIB_MIN_CHARS", "-1")]))), [Level::Fail, Level::Fail]);
        assert_eq!(levels(&settings(with(&[("KEYTIMEOUT", "40")]))), [Level::Info]);
        assert_eq!(levels(&settings(with(&[("WIB_SECRET_GUARD", "0")]))), [Level::Info]);
    }

    #[test]
    fn speed_and_secrets() {
        assert_eq!(speed(10, 100).level, Level::Ok);
        assert_eq!(speed(80, 100).level, Level::Warn);
        assert_eq!(secrets(0).level, Level::Ok);
        assert_eq!(secrets(2).level, Level::Warn);
    }
}
