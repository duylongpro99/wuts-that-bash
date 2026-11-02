use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DAY: i64 = 86_400;

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str, history: &[(i64, &str)]) -> Self {
        let dir = std::env::temp_dir().join(format!("wib-cli-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("data")).unwrap();
        let text: String = history
            .iter()
            .map(|(ts, cmd)| format!(": {ts}:0;{cmd}\n"))
            .collect();
        fs::write(dir.join("zsh_history"), text).unwrap();
        Sandbox { dir }
    }

    fn wib(&self, args: &[&str]) -> (String, i32) {
        let out = Command::new(env!("CARGO_BIN_EXE_wib"))
            .args(args)
            .env("WIB_ZSH_HISTORY", self.dir.join("zsh_history"))
            .env("WIB_DATA_DIR", self.dir.join("data"))
            .output()
            .unwrap();
        (String::from_utf8(out.stdout).unwrap(), out.status.code().unwrap_or(-1))
    }

    fn query(&self, period: &str, input: &str) -> Vec<String> {
        let (out, code) = self.wib(&["query", "--period", period, "--", input]);
        assert_eq!(code, 0);
        out.lines().map(str::to_string).collect()
    }

    fn data(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn period_filters_old_commands() {
    let n = now();
    let sb = Sandbox::new("period", &[(n - 10 * DAY, "kubectl get pods -A"), (n - DAY, "kubectl logs api")]);
    assert_eq!(sb.query("7d", "kub"), ["kubectl logs api"]);
    assert_eq!(sb.query("30d", "kub"), ["kubectl logs api", "kubectl get pods -A"]);
}

#[test]
fn fuzzy_initials_match() {
    let sb = Sandbox::new("fuzzy", &[(now() - 60, "docker compose up")]);
    assert_eq!(sb.query("30d", "dcup"), ["docker compose up"]);
}

#[test]
fn recorded_command_is_suggested_and_space_prefixed_is_not() {
    let sb = Sandbox::new("record", &[]);
    let ts = now().to_string();
    let (_, code) = sb.wib(&["record", "--ts", &ts, "--exit", "0", "--cwd", "/tmp", "--", "echo wib-marker"]);
    assert_eq!(code, 0);
    sb.wib(&["record", "--ts", &ts, "--exit", "0", "--cwd", "/tmp", "--", " echo secret"]);
    assert_eq!(sb.query("30d", "ec"), ["echo wib-marker"]);
    let log = fs::read_to_string(sb.data().join("data/history.tsv")).unwrap();
    assert!(!log.contains("secret"));
}

#[test]
fn same_execution_in_both_sources_counts_once() {
    let n = now();
    // "make new" ran once and is in both files (timestamps 1s apart);
    // "make old" ran twice but only in .zsh_history (a shell without wib).
    let sb = Sandbox::new(
        "overlap",
        &[(n - 5 * DAY, "make old"), (n - 4 * DAY, "make old"), (n - 60, "make new")],
    );
    let ts = (n - 59).to_string();
    sb.wib(&["record", "--ts", &ts, "--exit", "0", "--cwd", "/", "--", "make new"]);
    // Counted once, "make new" (1 use, recent: weight 4) beats "make old" (2 uses, old: 2 × 1).
    // Counted twice it would still win, so also check it isn't doubled against a 3-use command.
    let (out, _) = sb.wib(&["query", "--period", "30d", "--", "make"]);
    assert_eq!(out.lines().collect::<Vec<_>>(), ["make new", "make old"]);

    let sb = Sandbox::new(
        "overlap2",
        &[(n - 30, "cargo a"), (n - 31, "cargo a"), (n - 32, "cargo a"), (n - 60, "cargo b")],
    );
    for ts in [n - 60, n - 61] {
        let ts = ts.to_string();
        sb.wib(&["record", "--ts", &ts, "--exit", "0", "--cwd", "/", "--", "cargo b"]);
    }
    // cargo b: one run in both files (deduped) + one wib-only run = 2 < 3.
    let (out, _) = sb.wib(&["query", "--period", "30d", "--", "cargo"]);
    assert_eq!(out.lines().collect::<Vec<_>>(), ["cargo a", "cargo b"]);
}

#[test]
fn null_output_keeps_multiline_commands_intact() {
    let sb = Sandbox::new("null", &[]);
    let ts = now().to_string();
    sb.wib(&["record", "--ts", &ts, "--exit", "0", "--cwd", "/", "--", "for x in 1 2; do\n  echo $x\ndone"]);
    let (out, _) = sb.wib(&["query", "-0", "--", "for"]);
    assert_eq!(out, "for x in 1 2; do\n  echo $x\ndone\0");
    let (out, _) = sb.wib(&["query", "--", "for"]);
    assert_eq!(out, "for x in 1 2; do\\n  echo $x\\ndone\n");
}

#[test]
fn invalid_arguments_exit_with_2() {
    let sb = Sandbox::new("args", &[]);
    assert_eq!(sb.wib(&["query", "--period", "7x", "--", "ls"]).1, 2);
    assert_eq!(sb.wib(&["--bogus"]).1, 2);
    assert_eq!(sb.wib(&["init", "bash"]).1, 2);
}

#[test]
fn init_zsh_embeds_binary_path() {
    let sb = Sandbox::new("init", &[]);
    let (out, code) = sb.wib(&["init", "zsh"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("typeset -g _wib_bin='/"));
    assert!(out.contains("_wib_pre_redraw"));
}

fn log_lines(sb: &Sandbox) -> Vec<String> {
    let log = fs::read_to_string(sb.data().join("data/history.tsv")).unwrap_or_default();
    log.lines().map(str::to_string).collect()
}

fn record(sb: &Sandbox, ts: i64, cmd: &str, period: Option<&str>) -> i32 {
    let ts = ts.to_string();
    let mut args = vec!["record", "--ts", &ts, "--exit", "0", "--cwd", "/"];
    if let Some(p) = period {
        args.extend(["--period", p]);
    }
    args.extend(["--", cmd]);
    sb.wib(&args).1
}

#[test]
fn record_with_period_drops_old_log_entries() {
    let sb = Sandbox::new("compact", &[]);
    let n = now();
    for cmd in ["old a", "old b", "old c"] {
        record(&sb, n - 40 * DAY, cmd, None);
    }
    assert_eq!(log_lines(&sb).len(), 3);
    assert_eq!(record(&sb, n, "fresh", Some("30d")), 0);
    let lines = log_lines(&sb);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("\tfresh"));
}

#[test]
fn record_without_valid_period_keeps_old_entries() {
    let sb = Sandbox::new("keep", &[]);
    let n = now();
    record(&sb, n - 40 * DAY, "old", None);
    assert_eq!(record(&sb, n, "no period", None), 0);
    assert_eq!(record(&sb, n, "bad period", Some("7x")), 0);
    assert_eq!(record(&sb, n, "huge period", Some("9999999999999999d")), 0);
    assert_eq!(log_lines(&sb).len(), 4);
}

#[test]
fn concurrent_records_lose_nothing_while_compacting() {
    let sb = Sandbox::new("concurrent", &[]);
    let n = now();
    for i in 0..50 {
        record(&sb, n - 40 * DAY, &format!("old {i}"), None);
    }
    std::thread::scope(|s| {
        for w in 0..4 {
            let sb = &sb;
            s.spawn(move || {
                for i in 0..200 {
                    assert_eq!(record(sb, n, &format!("w{w} cmd {i}"), Some("30d")), 0);
                }
            });
        }
    });
    let lines = log_lines(&sb);
    assert_eq!(lines.len(), 800, "every new command kept, every old one dropped");
    for line in &lines {
        assert_eq!(line.split('\t').count(), 4, "malformed line: {line:?}");
    }
    let mut cmds: Vec<&str> = lines.iter().map(|l| l.rsplit('\t').next().unwrap()).collect();
    cmds.sort_unstable();
    cmds.dedup();
    assert_eq!(cmds.len(), 800);
}

fn wib_env(sb: &Sandbox, args: &[&str], env: &[(&str, &str)]) -> (Vec<String>, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_wib"))
        .args(args)
        .env("WIB_ZSH_HISTORY", sb.dir.join("zsh_history"))
        .env("WIB_DATA_DIR", sb.dir.join("data"))
        .env_remove("WIB_PERIOD")
        .env_remove("WIB_LIMIT")
        .envs(env.iter().copied())
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    (text.lines().map(str::to_string).collect(), out.status.code().unwrap_or(-1))
}

fn keyword_sandbox(name: &str) -> Sandbox {
    let n = now();
    Sandbox::new(
        name,
        &[
            (n - 3 * DAY, "docker pull nginx"),
            (n - 600, "git pull origin master"),
            (n - 500, "git push origin master"),
            (n - 400, "echo query"),
            (n - 300, "wib pull,master"),
        ],
    )
}

#[test]
fn bare_keywords_search_in_any_order() {
    let sb = keyword_sandbox("kw");
    let run = |args: &[&str]| wib_env(&sb, args, &[]);
    assert_eq!(run(&["pull"]), (vec!["git pull origin master".into(), "docker pull nginx".into()], 0));
    for args in [&["pull,master"][..], &["master", "pull"], &["master, pull"], &["--", "pull,master"]] {
        assert_eq!(run(args), (vec!["git pull origin master".to_string()], 0), "{args:?}");
    }
    assert_eq!(run(&["--", "query"]), (vec!["echo query".to_string()], 0));
    assert_eq!(run(&["pull,main"]), (vec![], 0));
}

#[test]
fn keyword_query_flag_for_the_widget() {
    let sb = keyword_sandbox("kwflag");
    let (out, code) = sb.wib(&["query", "--keywords", "-0", "--", "master pull"]);
    assert_eq!((out.as_str(), code), ("git pull origin master\0", 0));
    // Without the flag the input is matched in order, as before.
    assert!(sb.query("30d", "master pull").is_empty());
}

#[test]
fn bare_keywords_read_period_and_limit_from_env() {
    let sb = keyword_sandbox("kwenv");
    assert_eq!(wib_env(&sb, &["pull"], &[("WIB_LIMIT", "1")]).0, ["git pull origin master"]);
    assert_eq!(wib_env(&sb, &["pull"], &[("WIB_PERIOD", "1d")]).0, ["git pull origin master"]);
    assert_eq!(wib_env(&sb, &["pull"], &[("WIB_LIMIT", "x")]).1, 2);
}

#[test]
fn missing_keywords_exit_with_2() {
    let sb = keyword_sandbox("kwnone");
    assert_eq!(wib_env(&sb, &["--"], &[]).1, 2);
    assert_eq!(wib_env(&sb, &[","], &[]).1, 2);
}

fn record_run(sb: &Sandbox, ts: i64, cmd: &str, cwd: &str, exit: i32) {
    let (ts, exit) = (ts.to_string(), exit.to_string());
    let code = sb.wib(&["record", "--ts", &ts, "--exit", &exit, "--cwd", cwd, "--", cmd]).1;
    assert_eq!(code, 0);
}

fn lines(out: &str) -> Vec<&str> {
    out.lines().collect()
}

#[test]
fn cwd_prefers_commands_from_the_same_project() {
    let sb = Sandbox::new("cwd", &[]);
    let repo = sb.data().join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(repo.join("sub")).unwrap();
    let repo_s = repo.to_string_lossy().into_owned();
    let n = now();
    record_run(&sb, n - 100, "make test", &format!("{repo_s}/sub"), 0);
    for i in 0..3 {
        record_run(&sb, n - 50 - i, "make build", "/elsewhere", 0);
    }
    let (out, _) = sb.wib(&["query", "--cwd", &repo_s, "--", "make"]);
    assert_eq!(lines(&out), ["make test", "make build"]);
    // Any directory inside the project counts, since the git root is the scope.
    let (out, _) = sb.wib(&["query", "--cwd", &format!("{repo_s}/sub"), "--", "make"]);
    assert_eq!(lines(&out), ["make test", "make build"]);
    let (out, _) = sb.wib(&["query", "--", "make"]);
    assert_eq!(lines(&out), ["make build", "make test"]);
    // Bare keyword search reads $PWD.
    let (out, _) = wib_env(&sb, &["make"], &[("PWD", &repo_s)]);
    assert_eq!(out, ["make test", "make build"]);
    let (out, _) = wib_env(&sb, &["make"], &[("PWD", "/elsewhere")]);
    assert_eq!(out, ["make build", "make test"]);
}

#[test]
fn never_successful_commands_rank_last() {
    let sb = Sandbox::new("failed", &[]);
    let n = now();
    record_run(&sb, n - 60, "gti status", "/", 127);
    record_run(&sb, n - 60, "git status", "/", 0);
    record_run(&sb, n - 30, "gti status", "/", 127);
    let (out, _) = sb.wib(&["query", "--", "g"]);
    assert_eq!(lines(&out), ["git status", "gti status"]);
}

#[test]
fn forget_removes_a_command_from_every_lookup() {
    let n = now();
    let sb = Sandbox::new(
        "forget",
        &[
            (n - 60, "git push --force"),
            (n - 50, "git push origin"),
            (n - 40, "echo \"a b\""),
            (n - 35, "wib forget git push --force"),
            (n - 30, "printf a\\\nb"),
        ],
    );
    let (out, code) = sb.wib(&["forget", "git", "push", "--force"]);
    assert_eq!((out.as_str(), code), ("forgot: git push --force\n", 0));
    let (out, code) = sb.wib(&["forget", "git push --force"]);
    assert_eq!((out.as_str(), code), ("already forgotten: git push --force\n", 0));
    let file = sb.data().join("data/forget");
    assert_eq!(fs::read_to_string(&file).unwrap(), "git push --force\n");
    let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);

    assert_eq!(sb.query("30d", "git"), ["git push origin"]);
    assert_eq!(wib_env(&sb, &["push"], &[]).0, ["git push origin"]);
    let (out, _) = sb.wib(&["query", "--keywords", "--", "force"]);
    assert_eq!(out, "");
    // The `wib forget ...` line itself holds the forgotten command.
    assert!(sb.query("30d", "wib").is_empty());

    sb.wib(&["forget", "echo \"a b\""]);
    assert!(sb.query("30d", "echo").is_empty());
    // .zsh_history stores the newline as a trailing backslash.
    sb.wib(&["forget", "--", "printf a\nb"]);
    assert!(sb.query("30d", "printf").is_empty());
}

#[test]
fn forget_needs_a_command_and_double_dash_searches_the_word() {
    let n = now();
    let sb = Sandbox::new("forgetargs", &[(n - 60, "echo forget me")]);
    assert_eq!(sb.wib(&["forget"]).1, 2);
    assert_eq!(sb.wib(&["forget", "--", "  "]).1, 2);
    assert!(!sb.data().join("data/forget").exists());
    assert_eq!(wib_env(&sb, &["--", "forget"], &[]), (vec!["echo forget me".to_string()], 0));
}
