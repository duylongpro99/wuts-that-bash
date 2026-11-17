//! `wib setup zsh`: add the init line to `.zshrc` once, and the helpers that
//! `wib init zsh` and `wib doctor` share with it (where `.zshrc` is, which line
//! loads wib, which `wib` on `$PATH` is this binary, what a terminal host needs).

use crate::log_store;
use std::fs;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const INIT: &str = "eval \"$(wib init zsh)\"";

/// Every `wib` on $PATH, in order (relative entries skipped).
fn wibs_on_path() -> Vec<PathBuf> {
    let Some(path) = std::env::var_os("PATH") else { return Vec::new() };
    std::env::split_paths(&path).filter(|d| d.is_absolute()).map(|d| d.join("wib")).filter(|c| c.is_file()).collect()
}

/// The `wib` on $PATH that is this binary, as $PATH spells it (a symlink stays
/// a symlink), so a cached `wib init zsh` output survives an upgrade that
/// removes the versioned directory the symlink pointed into.
pub fn self_on_path() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?.canonicalize().ok()?;
    wibs_on_path().into_iter().find(|c| c.canonicalize().is_ok_and(|c| c == me))
}

/// `$ZDOTDIR/.zshrc` when `ZDOTDIR` is set and not empty, else `~/.zshrc`.
pub fn zshrc_path(zdotdir: Option<String>) -> PathBuf {
    zdotdir.filter(|d| !d.is_empty()).map_or_else(log_store::home, PathBuf::from).join(".zshrc")
}

/// `ZDOTDIR` from the environment, or else as zsh sets it for itself (often in
/// `~/.zshenv` without `export`, so a parent shell or bash never sees it).
pub fn zdotdir() -> Option<String> {
    if let Some(d) = std::env::var("ZDOTDIR").ok().filter(|d| !d.is_empty()) {
        return Some(d);
    }
    let out = std::process::Command::new("zsh")
        .args(["-c", "print -r -- \"wib-zdotdir=${ZDOTDIR-}\""])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let d = text.lines().rev().find_map(|l| l.strip_prefix("wib-zdotdir="))?;
    (!d.is_empty()).then(|| d.to_string())
}

/// `path` with the home directory shown as `~`.
pub fn shown(path: &Path) -> String {
    match path.strip_prefix(log_store::home()) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// The text of `.zshrc`, with bytes that are not UTF-8 replaced: only searched,
/// never written back, so a Latin-1 comment does not hide the init line.
pub fn read_rc(path: &Path) -> std::io::Result<String> {
    fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// 1-based number of the first line that is not a comment and runs `wib init zsh`.
pub fn init_line(text: &str) -> Option<usize> {
    text.lines()
        .enumerate()
        .find(|(_, l)| {
            let l = l.trim();
            !l.starts_with('#') && l.contains("wib init zsh")
        })
        .map(|(i, _)| i + 1)
}

/// What to do in a terminal that an app (an IDE, Orca) started with the
/// environment it had before wib was set up.
pub fn host_hint(get: impl Fn(&str) -> Option<String>) -> &'static str {
    if get("TERM_PROGRAM").as_deref() == Some("vscode") {
        "VS Code / Cursor: quit the app fully and open it again; it keeps the shell environment from when it started"
    } else if get("TERMINAL_EMULATOR").as_deref() == Some("JetBrains-JediTerm") {
        "JetBrains IDE: quit the IDE fully and open it again; it keeps the shell environment from when it started"
    } else {
        "terminals inside an app (Orca, an IDE): if a new one still lacks wib, quit that app fully and open it again"
    }
}

/// The lines `setup` appends, between markers.
pub fn block(path_line: Option<&str>) -> String {
    let path_line = path_line.map(|l| format!("{l}\n")).unwrap_or_default();
    format!("# >>> wib >>>\n{path_line}{INIT}\n# <<< wib <<<\n")
}

/// `export PATH=…` for the directory of this binary when no `wib` is on $PATH.
/// Another `wib` on $PATH (say Homebrew's, while this one is a download or a
/// build) is left in charge rather than shadowed for good.
/// `Err` when the directory cannot be written safely inside double quotes.
fn path_line() -> Result<Option<String>, String> {
    if !wibs_on_path().is_empty() {
        return Ok(None);
    }
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate wib: {e}"))?;
    let dir = exe.parent().ok_or("cannot locate wib's directory")?;
    let dir = dir.to_string_lossy();
    if dir.contains(['"', '$', '`', '\\']) {
        return Err(format!("the path of wib's directory ({dir}) has characters that need quoting by hand"));
    }
    let home = log_store::home();
    let home = home.to_string_lossy();
    let dir = match dir.strip_prefix(home.as_ref()) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("$HOME{rest}"),
        _ => dir.into_owned(),
    };
    Ok(Some(format!("export PATH=\"{dir}:$PATH\"")))
}

fn next_steps() {
    if self_on_path().is_none() {
        if let Some(other) = wibs_on_path().first() {
            println!("note: new shells will load {} (the first wib on PATH), not this binary", other.display());
        }
    }
    println!("this terminal: run  exec zsh");
    println!("{}", host_hint(|k| std::env::var(k).ok()));
}

/// `wib setup zsh`.
pub fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 1 || args[0] != "zsh" {
        return Err("usage: wib setup zsh".into());
    }
    let rc = zshrc_path(zdotdir());
    let name = shown(&rc);
    let manual = |why: String, lines: &str| format!("{why}\nadd these lines at the end of {name} yourself:\n\n{lines}");
    const SOME_DIR: &str = "export PATH=\"<dir of wib>:$PATH\"";
    // The lines to show when refusing; computing them must not itself fail.
    let lines = || block(path_line().unwrap_or_else(|_| Some(SOME_DIR.into())).as_deref());
    let euid = log_store::euid();

    let text = match fs::symlink_metadata(&rc) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // A new file belongs to whoever runs this; under sudo that would
            // leave a root-owned .zshrc in the user's home.
            let dir = rc.parent().unwrap_or(Path::new("/"));
            match fs::metadata(dir) {
                Ok(m) if m.is_dir() && m.uid() == euid => {}
                Ok(_) => return Err(manual(format!("{name} does not exist and its directory is not yours; not creating it"), &lines())),
                Err(e) => {
                    let why = format!("{name} does not exist and neither does its directory {} ({e}); is ZDOTDIR right?", dir.display());
                    return Err(manual(why, &lines()));
                }
            }
            String::new()
        }
        _ => {
            // Follows a symlink: the file it points to is what gets written.
            match fs::metadata(&rc) {
                Ok(m) if m.is_file() && m.uid() == euid => {}
                _ => return Err(manual(format!("{name} is not a regular file you own; not changing it"), &lines())),
            }
            read_rc(&rc).map_err(|e| manual(format!("cannot read {name}: {e}"), &lines()))?
        }
    };
    if let Some(n) = init_line(&text) {
        println!("already set up: {name}:{n}");
        next_steps();
        return Ok(());
    }

    let ours = block(path_line().map_err(|why| manual(why, &block(Some(SOME_DIR))))?.as_deref());
    let mut add = String::new();
    if !text.is_empty() {
        add.push_str(if text.ends_with('\n') { "\n" } else { "\n\n" });
    }
    add.push_str(&ours);
    fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&rc)
        .and_then(|mut f| f.write_all(add.as_bytes()))
        .map_err(|e| manual(format!("cannot write {name}: {e}"), &ours))?;
    let line = init_line(&(text + &add)).unwrap_or(0);
    println!("added wib to {name} (line {line})");
    next_steps();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_line_skips_comments() {
        assert_eq!(init_line("ls\n# eval \"$(wib init zsh)\"\n  eval \"$(wib init zsh)\"\n"), Some(3));
        assert_eq!(init_line("# wib init zsh\n"), None);
        assert_eq!(init_line(""), None);
    }

    #[test]
    fn host_hint_names_the_app() {
        let env = |k: &'static str, v: &'static str| move |name: &str| (name == k).then(|| v.to_string());
        assert!(host_hint(env("TERM_PROGRAM", "vscode")).starts_with("VS Code / Cursor"));
        assert!(host_hint(env("TERMINAL_EMULATOR", "JetBrains-JediTerm")).starts_with("JetBrains"));
        assert!(host_hint(env("TERM_PROGRAM", "iTerm.app")).contains("Orca"));
    }

    #[test]
    fn block_has_markers_and_optional_path() {
        assert_eq!(block(None), "# >>> wib >>>\neval \"$(wib init zsh)\"\n# <<< wib <<<\n");
        let b = block(Some("export PATH=\"$HOME/.local/bin:$PATH\""));
        let lines: Vec<&str> = b.lines().collect();
        assert_eq!(lines[1], "export PATH=\"$HOME/.local/bin:$PATH\"");
        assert_eq!(lines[2], INIT);
    }

    #[test]
    fn zshrc_follows_zdotdir() {
        assert_eq!(zshrc_path(Some("/z".into())), PathBuf::from("/z/.zshrc"));
        assert_eq!(zshrc_path(Some(String::new())), log_store::home().join(".zshrc"));
    }
}
