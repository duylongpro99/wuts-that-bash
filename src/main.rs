//! wib — suggest previously used shell commands while typing in zsh.
//!
//! Subcommands:
//! - `wib query [--period 30d] [--limit 8] [-0] -- <input>`
//!   Prints matching commands best-first. Default output is one command per
//!   line with embedded newlines shown as `\n` (for humans). With `-0`, each
//!   command is printed verbatim and terminated by NUL; the zsh widget uses this.
//! - `wib record --ts <unix> --exit <code> --cwd <dir> -- <cmd>`
//!   Appends a command to wib's own log (ignored if it starts with a space).
//! - `wib init zsh`  Prints the zsh integration script for `eval`.

mod entry;
mod log_store;
mod matcher;
mod rank;
mod zsh_history;

use entry::{parse_period, Entry};
use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_PERIOD: &str = "30d";
const DEFAULT_LIMIT: usize = 8;
const ZSH_SCRIPT: &str = include_str!("../shell/wib.zsh");

const USAGE: &str = "\
wib - suggest previously used commands from zsh history

Usage:
  wib query [--period 30d] [--limit 8] [-0] -- <input>
  wib record --ts <unix> --exit <code> --cwd <dir> -- <command>
  wib init zsh

Setup: add `eval \"$(wib init zsh)\"` to ~/.zshrc
Env:   WIB_PERIOD, WIB_LIMIT, WIB_MIN_CHARS, WIB_ZSH_HISTORY, WIB_DATA_DIR";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a: OsString| a.to_string_lossy().into_owned())
        .collect();
    let result = match args.first().map(String::as_str) {
        Some("query") => cmd_query(&args[1..]),
        Some("record") => cmd_record(&args[1..]),
        Some("init") => cmd_init(&args[1..]),
        Some("-V" | "--version") => {
            println!("wib {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("-h" | "--help") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{other}'\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
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
    let from_zsh: Vec<Entry> = match std::fs::read(zsh_history_path()) {
        Ok(raw) => zsh_history::parse_since(&raw, cutoff)
            .into_iter()
            .filter(|e| !(e.ts - 1..=e.ts + 1).any(|ts| own.contains(&(e.cmd.as_str(), ts))))
            .collect(),
        Err(_) => Vec::new(),
    };
    drop(own);
    entries.extend(from_zsh);
    entries
}

fn cmd_query(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args, &["--period", "--limit"], &["-0"])?;
    let period = parse_period(a.value("--period").unwrap_or(DEFAULT_PERIOD))?;
    let limit = match a.value("--limit") {
        Some(v) => v.parse().map_err(|_| format!("invalid --limit '{v}'"))?,
        None => DEFAULT_LIMIT,
    };
    let input = a.rest.join(" ");
    let now = now();
    let entries = load_entries(period, now);
    let results = rank::rank(&entries, &input, now, limit);

    let mut out = io::BufWriter::new(io::stdout().lock());
    for cmd in results {
        let written = if a.has("-0") {
            out.write_all(cmd.as_bytes()).and_then(|_| out.write_all(b"\0"))
        } else {
            writeln!(out, "{}", cmd.replace('\\', "\\\\").replace('\n', "\\n"))
        };
        if written.is_err() {
            return Ok(()); // reader went away (e.g. piped into head)
        }
    }
    let _ = out.flush();
    Ok(())
}

fn cmd_record(args: &[String]) -> Result<(), String> {
    let a = Args::parse(args, &["--ts", "--exit", "--cwd"], &[])?;
    let cmd = a.rest.join(" ");
    if !log_store::should_record(&cmd) {
        return Ok(());
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
    log_store::append(&log_store::data_dir(), &entry).map_err(|e| format!("cannot write log: {e}"))
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
