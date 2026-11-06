//! wib — suggest previously used shell commands while typing in zsh.
//!
//! Subcommands:
//! - `wib <keywords>` / `wib -- <keywords>`
//!   Prints commands containing every keyword (comma or space separated, any
//!   order), best-first. Period and limit come from `WIB_PERIOD` / `WIB_LIMIT`.
//!   Commands used in the current project (`$PWD`) rank first among equal matches.
//! - `wib query [--period 30d] [--limit 8] [--keywords] [--cwd <dir>] [-0] (--input-env | -- <input>)`
//!   Prints matching commands best-first. Default output is one command per
//!   line with embedded newlines shown as `\n` (for humans). With `-0`, each
//!   command is printed verbatim and terminated by NUL; the zsh widget uses this.
//!   With `--keywords`, `<input>` is a keyword search as above. With `--cwd`,
//!   commands run in that directory's project (git root, else the directory)
//!   rank first among equal matches. With `--input-env`, `<input>` is read from
//!   `$WIB_INPUT` so what is being typed never shows in `ps`.
//! - `wib forget <command>`  Never suggest `<command>` again (exact match after
//!   trimming). Stored in `<data dir>/forget`; `.zsh_history` is not touched.
//! - `wib record --ts <unix> --exit <code> --cwd <dir> [--period 30d] [--notice-id <pid>] (--stdin | -- <cmd>)`
//!   Appends a command to wib's own log (ignored if it starts with a space).
//!   With `--period`, entries older than the period are dropped from the log.
//!   A command that looks like it holds a secret is not saved; with
//!   `--notice-id`, `<data dir>/secret-notice.<pid>` tells that shell why.
//!   `--stdin` reads the command from stdin so it never shows in `ps`.
//!   `WIB_SECRET_GUARD=0` saves and suggests such commands anyway.
//! - `wib unforget [<command>]`  Without a command, lists forgotten commands
//!   (secrets masked). With one, suggests it again; exit 1 if it was not forgotten.
//! - `wib doctor`  Checks the installation (zsh version and options, key
//!   bindings, load order in `.zshrc`, data directory permissions, settings,
//!   speed) and says how to fix what is wrong; exit 1 if a check failed.
//! - `wib audit [--fix]`  Lists commands that look like they hold secrets in
//!   `.zsh_history`, wib's log and the forget file, masked; exit 1 if any. With
//!   `--fix`, removes them from wib's own files (never from `.zsh_history`).
//! - `wib slots --stdin [--period 30d]`  Reads a command line on stdin and
//!   prints `start end` (char offsets) for each word that earlier runs of the
//!   same command changed; the zsh widget on Ctrl-] jumps between them.
//! - `wib suggest-fix --stdin [--cwd <dir>] [--period 30d]`  Reads a command
//!   that failed with "command not found" and prints it with the program name
//!   corrected to one used before, or nothing. Arguments are never changed.
//! - `wib init zsh`  Prints the zsh integration script for `eval`.
//!
//! Commands that never succeeded rank after equal matches that did. Keywords
//! that are also subcommand names (audit, doctor, forget, init, query, record, slots, suggest-fix, unforget) need `wib --`.

mod audit;
mod doctor;
mod entry;
mod fix;
mod log_store;
mod matcher;
mod project;
mod rank;
mod secret;
mod slots;
mod zsh_history;

use entry::{parse_period, Entry};
use rank::RankContext;
use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_PERIOD: &str = "30d";
const DEFAULT_LIMIT: usize = 8;
const ZSH_SCRIPT: &str = include_str!("../shell/wib.zsh");

const USAGE: &str = "\
wib - suggest previously used commands from zsh history

Usage:
  wib <keywords>                  e.g. `wib pull,master` or `wib master pull`
  wib -- <keywords>               when a keyword is also a command name (audit, doctor, forget, init, query, record, slots, suggest-fix, unforget)
  wib query [--period 30d] [--limit 8] [--keywords] [--cwd <dir>] [-0] (--input-env | -- <input>)
  wib forget <command>            never suggest <command> again, e.g. `wib forget 'git push --force'`
  wib unforget [<command>]        list forgotten commands, or suggest <command> again
  wib doctor                      check the installation and say how to fix it
  wib audit [--fix]               find secrets in your history (shown masked); --fix removes them from wib's files
  wib record --ts <unix> --exit <code> --cwd <dir> [--period 30d] [--notice-id <pid>] (--stdin | -- <command>)
  wib init zsh

Setup: add `eval \"$(wib init zsh)\"` to ~/.zshrc
Env:   WIB_PERIOD, WIB_LIMIT, WIB_MIN_CHARS, WIB_ZSH_HISTORY, WIB_DATA_DIR, WIB_SECRET_GUARD";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a: OsString| a.to_string_lossy().into_owned())
        .collect();
    // `Ok(true)`: the command ran and found a problem (exit 1).
    let done = |r: Result<(), String>| r.map(|()| false);
    let result = match args.first().map(String::as_str) {
        Some("query") => done(cmd_query(&args[1..])),
        Some("record") => done(cmd_record(&args[1..])),
        Some("init") => done(cmd_init(&args[1..])),
        Some("forget") => done(cmd_forget(&args[1..])),
        Some("unforget") => cmd_unforget(&args[1..]),
        Some("doctor") => cmd_doctor(&args[1..]),
        Some("audit") => cmd_audit(&args[1..]),
        Some("slots") => done(cmd_slots(&args[1..])),
        Some("suggest-fix") => done(cmd_suggest_fix(&args[1..])),
        Some("-V" | "--version") => {
            println!("wib {}", env!("CARGO_PKG_VERSION"));
            Ok(false)
        }
        Some("-h" | "--help") | None => {
            println!("{USAGE}");
            Ok(false)
        }
        Some("--") => done(cmd_keywords(&args[1..])),
        Some(other) if other.starts_with('-') => Err(format!("unknown option '{other}'\n\n{USAGE}")),
        Some(_) => done(cmd_keywords(&args)),
    };
    match result {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(1),
        Err(msg) => {
            eprintln!("wib: {msg}");
            ExitCode::from(2)
        }
    }
}

/// Parsed `--flag value` options plus everything after `--`.
struct Args {
    opts: Vec<(String, Option<String>)>,
    rest: Vec<String>,
}

impl Args {
    fn parse(args: &[String], value_flags: &[&str], bool_flags: &[&str]) -> Result<Self, String> {
        let mut opts = Vec::new();
        let mut i = 0;
        while i < args.len() {
            let a = &args[i];
            if a == "--" {
                return Ok(Args { opts, rest: args[i + 1..].to_vec() });
            }
            if value_flags.contains(&a.as_str()) {
                let v = args.get(i + 1).ok_or_else(|| format!("{a} needs a value"))?;
                opts.push((a.clone(), Some(v.clone())));
                i += 2;
            } else if bool_flags.contains(&a.as_str()) {
                opts.push((a.clone(), None));
                i += 1;
            } else {
                // First positional argument starts the payload.
                return Ok(Args { opts, rest: args[i..].to_vec() });
            }
        }
        Ok(Args { opts, rest: Vec::new() })
    }

    fn value(&self, flag: &str) -> Option<&str> {
        self.opts.iter().rev().find(|(f, _)| f == flag).and_then(|(_, v)| v.as_deref())
    }

    fn has(&self, flag: &str) -> bool {
        self.opts.iter().any(|(f, _)| f == flag)
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn zsh_history_path() -> PathBuf {
    log_store::env_path("WIB_ZSH_HISTORY").unwrap_or_else(|| log_store::home().join(".zsh_history"))
}

/// Entries from both sources within `period_secs`. Both record the start time
/// of a command, so a `.zsh_history` entry with the same command within a
/// second of a wib entry is the same execution and is counted once. Commands
/// from shells without wib loaded still come through `.zsh_history`.
fn load_entries(period_secs: i64, now: i64) -> Vec<Entry> {
    let cutoff = now - period_secs;
    let mut entries = log_store::load(&log_store::data_dir());
    entries.retain(|e| e.ts >= cutoff);
    let own: HashSet<(&str, i64)> = entries.iter().map(|e| (e.cmd.as_str(), e.ts)).collect();
    let from_zsh: Vec<Entry> = match zsh_history::read_since(&zsh_history_path(), cutoff) {
        Ok(found) => found
            .into_iter()
            .filter(|e| !(e.ts - 1..=e.ts + 1).any(|ts| own.contains(&(e.cmd.as_str(), ts))))
            .collect(),
        Err(_) => Vec::new(),
    };
    drop(own);
    entries.extend(from_zsh);
    entries
}

/// `wib <keywords>`: a keyword query with period and limit from the environment,
/// preferring commands from the project of the current directory.
fn cmd_keywords(keywords: &[String]) -> Result<(), String> {
    let input = keywords.join(" ");
    if input.split(|c: char| c == ',' || c.is_whitespace()).all(str::is_empty) {
        return Err(format!("no keywords given\n\n{USAGE}"));
    }
    let env = |name: &str, default: &str| {
        std::env::var(name).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| default.to_string())
    };
    // `$PWD` is the logical path, as recorded by the shell hook.
    let cwd = log_store::env_path("PWD")
        .or_else(|| std::env::current_dir().ok())
        .map(|p| p.to_string_lossy().into_owned());
    let mut args: Vec<String> = cwd.into_iter().flat_map(|d| ["--cwd".to_string(), d]).collect();
    args.extend([
        "--keywords".to_string(),
        "--period".to_string(),
        env("WIB_PERIOD", DEFAULT_PERIOD),
        "--limit".to_string(),
        env("WIB_LIMIT", &DEFAULT_LIMIT.to_string()),
        "--".to_string(),
        input,
    ]);
    cmd_query(&args)
}

fn cmd_query(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args, &["--period", "--limit", "--cwd"], &["-0", "--keywords", "--input-env"])?;
    let period = parse_period(a.value("--period").unwrap_or(DEFAULT_PERIOD))?;
    let limit = match a.value("--limit") {
        Some(v) => v.parse().map_err(|_| format!("invalid --limit '{v}'"))?,
        None => DEFAULT_LIMIT,
    };
    let input = if a.has("--input-env") {
        std::env::var_os("WIB_INPUT").map(|v| v.to_string_lossy().into_owned()).unwrap_or_default()
    } else {
        a.rest.join(" ")
    };
    let now = now();
    let entries = load_entries(period, now);
    let scope = a.value("--cwd").map(|cwd| project::scope(cwd, &log_store::home()));
    let forgotten = log_store::load_forgotten(&log_store::data_dir());
    let ctx = RankContext { scope: scope.as_deref(), forgotten: Some(&forgotten), secret_guard: secret::guard_enabled() };
    let results = if a.has("--keywords") {
        rank::rank_keywords(&entries, &input, now, limit, ctx)
    } else {
        rank::rank(&entries, &input, now, limit, ctx)
    };

    let mut out = io::BufWriter::new(io::stdout().lock());
    for cmd in results {
        let written = if a.has("-0") {
            out.write_all(cmd.as_bytes()).and_then(|_| out.write_all(b"\0"))
        } else {
            writeln!(out, "{}", printable(cmd))
        };
        if written.is_err() {
            return Ok(()); // reader went away (e.g. piped into head)
        }
    }
    let _ = out.flush();
    Ok(())
}

/// A command as one terminal line: newlines become `\n`, and control
/// characters other than tab become `\xHH`, so a history entry holding an
/// escape sequence is shown, not interpreted by the terminal.
fn printable(cmd: &str) -> String {
    let mut out = String::with_capacity(cmd.len());
    for c in cmd.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push('\t'),
            c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// The command to act on: all of stdin with `--stdin` (so it never shows in
/// the process list), else the words after the options.
fn command_arg(a: &Args) -> Result<String, String> {
    if !a.has("--stdin") {
        return Ok(a.rest.join(" "));
    }
    let mut raw = Vec::new();
    io::stdin().read_to_end(&mut raw).map_err(|e| format!("cannot read stdin: {e}"))?;
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

fn cmd_record(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args, &["--ts", "--exit", "--cwd", "--period", "--notice-id"], &["--stdin"])?;
    let notice_id = a.value("--notice-id");
    if notice_id.is_some_and(|id| id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit())) {
        return Err("--notice-id must be a number".into());
    }
    let cmd = command_arg(&a)?;
    if !log_store::should_record(&cmd) {
        return Ok(());
    }
    // A command holding a secret is not saved; the shell that ran it is told why.
    if secret::guard_enabled() {
        if let Some(found) = secret::detect(&cmd) {
            if let Some(id) = notice_id {
                log_store::write_secret_notice(&log_store::data_dir(), id, found.kind.label())
                    .map_err(|e| format!("cannot write notice: {e}"))?;
            }
            return Ok(());
        }
    }
    let ts = match a.value("--ts") {
        Some(v) => v.parse().map_err(|_| format!("invalid --ts '{v}'"))?,
        None => now(),
    };
    let entry = Entry {
        ts,
        cmd: cmd.trim_end().to_string(),
        cwd: a.value("--cwd").map(str::to_string),
        exit: a.value("--exit").and_then(|v| v.parse().ok()),
    };
    // A bad --period must not lose the command; it only skips the cleanup.
    let now = now();
    let keep_since = a
        .value("--period")
        .and_then(|p| parse_period(p).ok())
        .map(|secs| now.saturating_sub(secs));
    log_store::append(&log_store::data_dir(), &entry, keep_since)
        .map_err(|e| format!("cannot write log: {e}"))
}

/// `wib forget <command>`: words are joined with single spaces, so quote a
/// command whose spacing matters.
fn cmd_forget(args: &[String]) -> Result<(), String> {
    let words = match args.first().map(String::as_str) {
        Some("--") => &args[1..],
        _ => args,
    };
    let cmd = words.join(" ");
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Err(format!("no command to forget\n\n{USAGE}"));
    }
    let added = log_store::forget(&log_store::data_dir(), cmd).map_err(|e| format!("cannot write forget file: {e}"))?;
    let shown = audit::shown(cmd);
    if added {
        println!("forgot: {shown}");
    } else {
        println!("already forgotten: {shown}");
    }
    Ok(())
}

/// `wib slots --stdin [--period 30d]`: the buffer on stdin; prints one
/// `start end` line (0-based char offsets, end exclusive) per slot.
fn cmd_slots(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args, &["--period"], &["--stdin"])?;
    let period = parse_period(a.value("--period").unwrap_or(DEFAULT_PERIOD))?;
    let buffer = command_arg(&a)?;
    let entries = load_entries(period, now());
    let forgotten = log_store::load_forgotten(&log_store::data_dir());
    let ctx = RankContext { scope: None, forgotten: Some(&forgotten), secret_guard: secret::guard_enabled() };
    let mut out = io::stdout().lock();
    for (start, end) in slots::find(&buffer, &entries, ctx) {
        let _ = writeln!(out, "{start} {end}");
    }
    Ok(())
}

/// `wib suggest-fix --stdin [--cwd <dir>] [--period 30d]`: the command that
/// was not found on stdin; prints it with the program name corrected, or nothing.
fn cmd_suggest_fix(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args, &["--period", "--cwd"], &["--stdin"])?;
    let period = parse_period(a.value("--period").unwrap_or(DEFAULT_PERIOD))?;
    let cmd = command_arg(&a)?;
    let now = now();
    let entries = load_entries(period, now);
    let scope = a.value("--cwd").map(|cwd| project::scope(cwd, &log_store::home()));
    let forgotten = log_store::load_forgotten(&log_store::data_dir());
    let ctx = RankContext { scope: scope.as_deref(), forgotten: Some(&forgotten), secret_guard: secret::guard_enabled() };
    if let Some(fixed) = fix::suggest(&cmd, &entries, now, ctx) {
        println!("{fixed}");
    }
    Ok(())
}

/// `wib audit [--fix]`: exit 1 when a secret was found.
fn cmd_audit(args: &[String]) -> Result<bool, String> {
    let fix = match args {
        [] => false,
        [flag] if flag == "--fix" => true,
        _ => return Err(format!("usage: wib audit [--fix]\n\n{USAGE}")),
    };
    audit::run(&zsh_history_path(), fix)
}

/// `wib unforget`: list what was forgotten (secrets masked). `wib unforget
/// <command>`: suggest it again; exit 1 if it was not forgotten.
fn cmd_unforget(args: &[String]) -> Result<bool, String> {
    let words = match args.first().map(String::as_str) {
        Some("--") => &args[1..],
        _ => args,
    };
    let dir = log_store::data_dir();
    let cmd = words.join(" ");
    let cmd = cmd.trim();
    if cmd.is_empty() {
        let mut forgotten: Vec<String> = log_store::load_forgotten(&dir).into_iter().collect();
        forgotten.sort();
        if forgotten.is_empty() {
            println!("nothing forgotten");
        }
        for (i, cmd) in forgotten.iter().enumerate() {
            println!("{:>3}  {}", i + 1, audit::shown(cmd));
        }
        return Ok(false);
    }
    let removed = log_store::unforget(&dir, cmd).map_err(|e| format!("cannot write forget file: {e}"))?;
    if removed {
        println!("unforgot: {}", audit::shown(cmd));
    } else {
        println!("not forgotten: {}", audit::shown(cmd));
    }
    Ok(!removed)
}

/// `wib doctor`: exit 1 when a check failed.
fn cmd_doctor(args: &[String]) -> Result<bool, String> {
    if !args.is_empty() {
        return Err(format!("usage: wib doctor\n\n{USAGE}"));
    }
    let period = std::env::var("WIB_PERIOD").ok().filter(|v| !v.is_empty());
    let period = parse_period(period.as_deref().unwrap_or(DEFAULT_PERIOD)).unwrap_or(30 * 86_400);
    Ok(doctor::run(&zsh_history_path(), period, now()))
}

fn cmd_init(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("zsh") => {
            let exe = std::env::current_exe().map_err(|e| format!("cannot locate wib: {e}"))?;
            // Single-quote the path for zsh.
            let quoted = exe.to_string_lossy().replace('\'', r"'\''");
            print!("typeset -g _wib_bin='{quoted}'\n{ZSH_SCRIPT}");
            Ok(())
        }
        _ => Err("usage: wib init zsh".into()),
    }
}
