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
fn text_output_shows_control_characters_instead_of_sending_them() {
    let sb = Sandbox::new("ctrl", &[]);
    let ts = now().to_string();
    sb.wib(&["record", "--ts", &ts, "--exit", "0", "--cwd", "/", "--", "echo \x1b]0;pwned\x07\tdone\u{9b}"]);
    let (out, _) = sb.wib(&["query", "--", "echo"]);
    assert_eq!(out, "echo \\x1b]0;pwned\\x07\tdone\\x9b\n");
    let (out, _) = sb.wib(&["query", "-0", "--", "echo"]);
    assert_eq!(out, "echo \x1b]0;pwned\x07\tdone\u{9b}\0");
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

/// Run wib with `input` on stdin.
fn wib_stdin(sb: &Sandbox, args: &[&str], input: &str, env: &[(&str, &str)]) -> (String, i32) {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_wib"))
        .args(args)
        .env("WIB_ZSH_HISTORY", sb.dir.join("zsh_history"))
        .env("WIB_DATA_DIR", sb.dir.join("data"))
        .env_remove("WIB_SECRET_GUARD")
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    (String::from_utf8(out.stdout).unwrap(), out.status.code().unwrap_or(-1))
}

const BEARER: &str = "curl -s -H 'Authorization: Bearer Zm9vYmFyYmF6cXV4cXV1eDEyMzQ1Njc4OTA' localhost:1";

fn record_stdin(sb: &Sandbox, cmd: &str, env: &[(&str, &str)]) -> i32 {
    let ts = now().to_string();
    let args = ["record", "--stdin", "--notice-id", "4242", "--ts", &ts, "--exit", "0", "--cwd", "/"];
    wib_stdin(sb, &args, cmd, env).1
}

#[test]
fn secrets_are_not_recorded_and_the_shell_is_told() {
    let sb = Sandbox::new("secret-record", &[]);
    assert_eq!(record_stdin(&sb, BEARER, &[]), 0);
    assert!(log_lines(&sb).is_empty());
    let notice = sb.data().join("data/secret-notice.4242");
    assert_eq!(fs::read_to_string(&notice).unwrap(), "secret header");
    assert_eq!(fs::metadata(&notice).unwrap().permissions().mode() & 0o777, 0o600);

    // A plain command is still recorded, from stdin, verbatim.
    assert_eq!(record_stdin(&sb, "echo plain\nnext", &[]), 0);
    assert_eq!(log_lines(&sb).len(), 1);
    assert_eq!(sb.query("30d", "echo"), ["echo plain\\nnext"]);
}

#[test]
fn secret_guard_can_be_turned_off() {
    let sb = Sandbox::new("secret-off", &[]);
    assert_eq!(record_stdin(&sb, BEARER, &[("WIB_SECRET_GUARD", "0")]), 0);
    assert_eq!(log_lines(&sb).len(), 1);
    assert!(!sb.data().join("data/secret-notice.4242").exists());
    let (out, _) = wib_stdin(&sb, &["query", "--", "curl"], "", &[("WIB_SECRET_GUARD", "0")]);
    assert_eq!(out.lines().count(), 1);
}

#[test]
fn secrets_in_zsh_history_are_never_suggested() {
    let n = now();
    let sb = Sandbox::new("secret-query", &[(n - 60, BEARER), (n - 50, "curl -s localhost:2")]);
    assert_eq!(sb.query("30d", "cur"), ["curl -s localhost:2"]);
    let (out, _) = sb.wib(&["query", "--keywords", "--", "bearer"]);
    assert_eq!(out, "");
}

#[test]
fn query_input_can_come_from_the_environment() {
    let sb = keyword_sandbox("input-env");
    let (out, _) = wib_stdin(&sb, &["query", "-0", "--input-env"], "", &[("WIB_INPUT", "git pu")]);
    assert_eq!(out, "git push origin master\0git pull origin master\0");
    let (out, _) = wib_stdin(&sb, &["query", "--keywords", "--input-env"], "", &[("WIB_INPUT", "master pull")]);
    assert_eq!(out, "git pull origin master\n");
}

#[test]
fn bad_notice_id_is_refused() {
    let sb = Sandbox::new("secret-id", &[]);
    let (_, code) = wib_stdin(&sb, &["record", "--stdin", "--notice-id", "../x"], "ls", &[]);
    assert_eq!(code, 2);
}

#[test]
fn stale_notices_are_cleaned_up() {
    let sb = Sandbox::new("secret-stale", &[]);
    record_stdin(&sb, "echo ok", &[]); // creates the data directory
    let old = sb.data().join("data/secret-notice.1");
    fs::write(&old, "secret header").unwrap();
    let two_days_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * DAY as u64);
    fs::File::options().write(true).open(&old).unwrap().set_modified(two_days_ago).unwrap();
    let fresh = sb.data().join("data/secret-notice.2");
    fs::write(&fresh, "secret header").unwrap();
    record_stdin(&sb, BEARER, &[]);
    assert!(!old.exists());
    assert!(fresh.exists());
}

const GH_SECRET: &str = "QzXvKwPqJmRtYbNcLdHfGsZxWvQpKmJt";
const DB_SECRET: &str = "Pq7vXw9zKj";

fn secret_sandbox(name: &str) -> Sandbox {
    let n = now();
    let gh = format!("export GITHUB_TOKEN=ghp_{GH_SECRET}");
    let db = format!("psql postgres://app:{DB_SECRET}@db.local/app");
    Sandbox::new(name, &[(n - 60, gh.as_str()), (n - 50, "git status"), (n - 40, db.as_str())])
}

#[test]
fn audit_lists_secrets_without_showing_them() {
    let sb = secret_sandbox("audit");
    let (out, code) = sb.wib(&["audit"]);
    assert_eq!(code, 1);
    let found: Vec<&str> = out.lines().filter(|l| l.contains("zsh_history:")).collect();
    assert_eq!(found.len(), 2, "{out}");
    assert!(found[0].contains("zsh_history:1") && found[0].contains("GitHub token") && found[0].ends_with("GITHUB_TOKEN=ghp_…"));
    assert!(found[1].contains("zsh_history:3") && found[1].ends_with("postgres://app:…@db.local/app"));
    for secret in [GH_SECRET, DB_SECRET] {
        let chars: Vec<char> = secret.chars().collect();
        for w in chars.windows(3) {
            let part: String = w.iter().collect();
            assert!(!out.contains(&part), "output shows {part:?}:\n{out}");
        }
    }
    assert!(out.contains("2 commands look like they contain secrets"));
}

#[test]
fn audit_of_a_clean_history_exits_0() {
    let sb = Sandbox::new("audit-clean", &[(now() - 60, "git status")]);
    assert_eq!(sb.wib(&["audit"]), ("no secrets found\n".to_string(), 0));
    assert_eq!(sb.wib(&["audit", "--bogus"]).1, 2);
}

#[test]
fn audit_fix_cleans_wibs_files_and_never_touches_zsh_history() {
    let sb = secret_sandbox("audit-fix");
    let off = [("WIB_SECRET_GUARD", "0")];
    record_stdin(&sb, &format!("mysql -u root -p{DB_SECRET}"), &off);
    record_stdin(&sb, "make build", &off);
    sb.wib(&["forget", &format!("curl -u me:{DB_SECRET} host")]);
    sb.wib(&["forget", "git push --force"]);
    let zsh_before = fs::read(sb.data().join("zsh_history")).unwrap();

    let (out, code) = sb.wib(&["audit", "--fix"]);
    assert_eq!(code, 1);
    assert!(out.contains("history.tsv:1") && out.contains("forget:1"), "{out}");
    assert!(out.contains("removed 1 line from") && out.contains("2 lines in"), "{out}");
    assert!(!out.contains(DB_SECRET));

    let log = log_lines(&sb);
    assert_eq!(log.len(), 1);
    assert!(log[0].ends_with("\tmake build"));
    let forget = sb.data().join("data/forget");
    assert_eq!(fs::read_to_string(&forget).unwrap(), "git push --force\n");
    for file in ["data/history.tsv", "data/forget"] {
        let mode = fs::metadata(sb.data().join(file)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{file}");
    }
    assert_eq!(fs::read(sb.data().join("zsh_history")).unwrap(), zsh_before);
    // Only .zsh_history still holds them.
    let (out, _) = sb.wib(&["audit"]);
    assert_eq!(out.lines().filter(|l| l.contains("zsh_history:")).count(), 2);
    assert!(!out.contains("history.tsv:") && !out.contains("forget:"));
}

#[test]
fn audit_fix_keeps_forgotten_secrets_hidden_when_the_guard_is_off() {
    let n = now();
    let cmd = format!("curl -u me:{DB_SECRET} host");
    let sb = Sandbox::new("audit-fix-off", &[(n - 60, cmd.as_str())]);
    sb.wib(&["forget", &cmd]);
    let off = [("WIB_SECRET_GUARD", "0")];
    let (out, _) = wib_stdin(&sb, &["audit", "--fix"], "", &off);
    assert!(out.contains("kept"), "{out}");
    assert!(fs::read_to_string(sb.data().join("data/forget")).unwrap().contains("curl -u me:"));
    let (out, _) = wib_stdin(&sb, &["query", "--", "curl"], "", &off);
    assert_eq!(out, "");
}

#[test]
fn forget_prints_the_command_masked() {
    let sb = Sandbox::new("forget-mask", &[]);
    let (out, _) = sb.wib(&["forget", &format!("curl -u me:{DB_SECRET} host")]);
    assert_eq!(out, "forgot: curl -u me:… host\n");
}

#[test]
fn audit_fix_while_recording_loses_nothing() {
    let sb = Sandbox::new("audit-race", &[]);
    let n = now();
    record_stdin(&sb, &format!("mysql -p{DB_SECRET}"), &[("WIB_SECRET_GUARD", "0")]);
    std::thread::scope(|s| {
        for w in 0..3 {
            let sb = &sb;
            s.spawn(move || {
                for i in 0..60 {
                    assert_eq!(record(sb, n, &format!("w{w} cmd {i}"), None), 0);
                }
            });
        }
        let sb = &sb;
        s.spawn(move || {
            for _ in 0..20 {
                sb.wib(&["audit", "--fix"]);
            }
        });
    });
    let lines = log_lines(&sb);
    assert_eq!(lines.len(), 180, "every plain command kept, the secret dropped");
    assert!(!lines.iter().any(|l| l.contains(DB_SECRET)));
}

#[test]
fn slots_print_char_offsets_of_the_parts_that_changed() {
    let n = now();
    // The sandbox points WIB_ZSH_HISTORY at its own file, as the zsh widget does.
    let sb = Sandbox::new("slots", &[(n - 60, "docker run -p 9090:80 nginx:1.24"), (n - 50, "kubectl logs pod-b")]);
    let (out, code) = wib_stdin(&sb, &["slots", "--stdin"], "docker run -p 8080:80 nginx:1.25", &[]);
    assert_eq!((out.as_str(), code), ("14 21\n22 32\n", 0));
    let (out, _) = wib_stdin(&sb, &["slots", "--stdin", "--period", "30d"], "kubectl logs pod-á", &[]);
    assert_eq!(out, "13 18\n");
    let (out, code) = wib_stdin(&sb, &["slots", "--stdin"], "ls -la", &[]);
    assert_eq!((out.as_str(), code), ("", 0));
    // A forgotten run is not a sibling.
    sb.wib(&["forget", "kubectl logs pod-b"]);
    let (out, _) = wib_stdin(&sb, &["slots", "--stdin"], "kubectl logs pod-a", &[]);
    assert_eq!(out, "");
}

#[test]
fn suggest_fix_corrects_the_program_name() {
    let sb = Sandbox::new("suggest-fix", &[]);
    record_run(&sb, now() - 60, "echo hi", "/", 0);
    let (out, code) = wib_stdin(&sb, &["suggest-fix", "--stdin", "--cwd", "/"], "ehco hi", &[]);
    assert_eq!((out.as_str(), code), ("echo hi\n", 0));
    // Arguments are kept as typed, never corrected.
    let (out, _) = wib_stdin(&sb, &["suggest-fix", "--stdin"], "ehco  'h i' x", &[]);
    assert_eq!(out, "echo  'h i' x\n");
    let (out, code) = wib_stdin(&sb, &["suggest-fix", "--stdin"], "qqqq", &[]);
    assert_eq!((out.as_str(), code), ("", 0));
}

#[test]
fn unforget_brings_a_command_back_and_lists_masked() {
    let n = now();
    let sb = Sandbox::new("unforget", &[(n - 60, "git push --force"), (n - 50, "git status")]);
    assert_eq!(sb.wib(&["unforget"]), ("nothing forgotten\n".to_string(), 0));
    sb.wib(&["forget", "git push --force"]);
    sb.wib(&["forget", &format!("export GITHUB_TOKEN=ghp_{GH_SECRET}")]);
    let (out, code) = sb.wib(&["unforget"]);
    assert_eq!(code, 0);
    assert!(out.contains("GITHUB_TOKEN=ghp_…") && out.contains("git push --force"), "{out}");
    assert!(!out.contains(&GH_SECRET[..3]) && !out.contains(&GH_SECRET[10..13]), "{out}");

    assert_eq!(sb.query("30d", "git"), ["git status"]);
    assert_eq!(sb.wib(&["unforget", "git", "push", "--force"]), ("unforgot: git push --force\n".to_string(), 0));
    assert_eq!(sb.query("30d", "git"), ["git status", "git push --force"]);
    assert_eq!(sb.wib(&["unforget", "--", "git push --force"]), ("not forgotten: git push --force\n".to_string(), 1));
    let mode = fs::metadata(sb.data().join("data/forget")).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn forget_and_unforget_at_once_lose_nothing() {
    let sb = Sandbox::new("unforget-race", &[]);
    for i in 0..40 {
        sb.wib(&["forget", &format!("keep {i}")]);
    }
    std::thread::scope(|s| {
        let sb = &sb;
        s.spawn(move || {
            for i in 0..40 {
                sb.wib(&["forget", &format!("new {i}")]);
            }
        });
        s.spawn(move || {
            for i in 0..40 {
                sb.wib(&["unforget", &format!("keep {i}")]);
            }
        });
    });
    let text = fs::read_to_string(sb.data().join("data/forget")).unwrap();
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort_unstable();
    let mut expected: Vec<String> = (0..40).map(|i| format!("new {i}")).collect();
    expected.sort_unstable();
    assert_eq!(lines, expected);
}

/// `wib doctor` with the shell state the zsh function would pass.
fn doctor(sb: &Sandbox, zshrc: &str) -> (String, i32) {
    let home = sb.data().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join(".zshrc"), zshrc).unwrap();
    let state = "zsh_version=5.9\nextended_history=on\nKEYTIMEOUT=1\n\
                 bind:^[[B=_wib_down\nbind:^[OB=_wib_down\nbind:^[[Z=_wib_word\n\
                 bind:^[=_wib_dismiss\nbind:^G=_wib_dismiss\nbind:^]=_wib_slot\n";
    let home = home.to_string_lossy().into_owned();
    wib_stdin(sb, &["doctor"], "", &[("HOME", &home), ("ZDOTDIR", ""), ("_WIB_DOCTOR_STATE", state)])
}

#[test]
fn doctor_warns_about_load_order_and_fails_on_a_shared_data_dir() {
    let sb = Sandbox::new("doctor", &[(now() - 60, "git status")]);
    let good = "source $ZSH/oh-my-zsh.sh\neval \"$(wib init zsh)\"\n";
    let (out, code) = doctor(&sb, good);
    assert_eq!(code, 0, "{out}");
    assert!(!out.lines().any(|l| l.starts_with("fail") || l.starts_with("warn")), "{out}");

    let (out, code) = doctor(&sb, "eval \"$(wib init zsh)\"\nsource $ZSH/oh-my-zsh.sh\n");
    assert_eq!(code, 0);
    assert!(out.lines().any(|l| l.starts_with("warn") && l.contains("oh-my-zsh.sh")), "{out}");

    let data = sb.data().join("data");
    fs::set_permissions(&data, fs::Permissions::from_mode(0o777)).unwrap();
    let (out, code) = doctor(&sb, good);
    assert_eq!(code, 1);
    assert!(out.contains("fail  ") && out.contains("chmod 700"), "{out}");
}

/// Runs wib with ZDOTDIR and the terminal-host variables cleared, then `env` applied.
fn run_wib(args: &[&str], env: &[(&str, &str)]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_wib"))
        .args(args)
        .env_remove("ZDOTDIR")
        .env_remove("TERM_PROGRAM")
        .env_remove("TERMINAL_EMULATOR")
        .env_remove("_WIB_DOCTOR_STATE")
        .envs(env.iter().copied())
        .output()
        .unwrap();
    (String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap(), out.status.code().unwrap_or(-1))
}

#[test]
fn doctor_outside_a_loaded_shell_says_why() {
    let sb = Sandbox::new("doctor-unloaded", &[]);
    let home = sb.data().join("home");
    fs::create_dir_all(&home).unwrap();
    let home_s = home.to_string_lossy().into_owned();
    let hist = sb.data().join("zsh_history").to_string_lossy().into_owned();
    let data = sb.data().join("data").to_string_lossy().into_owned();
    let run = |zshrc: &str| {
        fs::write(home.join(".zshrc"), zshrc).unwrap();
        let env = [("WIB_ZSH_HISTORY", hist.as_str()), ("WIB_DATA_DIR", &data), ("HOME", &home_s), ("TERM_PROGRAM", "vscode")];
        run_wib(&["doctor"], &env).0
    };
    let out = run("ls\neval \"$(wib init zsh)\"\n");
    let warn = out.lines().find(|l| l.contains("did not come from a shell that loaded it")).unwrap_or_else(|| panic!("{out}"));
    assert!(warn.starts_with("warn") && warn.contains("~/.zshrc:2"), "{out}");
    assert!(out.contains("quit the app fully"), "{out}");
    let out = run("");
    assert!(out.contains("no `wib init zsh` line") && out.contains("wib setup zsh"), "{out}");
}

/// A `bin/wib` symlink to the built binary inside the sandbox.
fn wib_on_path(sb: &Sandbox) -> PathBuf {
    let bin = sb.data().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let link = bin.join("wib");
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_wib"), &link).unwrap();
    bin
}

#[test]
fn init_zsh_uses_the_path_symlink() {
    let sb = Sandbox::new("init-path", &[]);
    let bin = wib_on_path(&sb);
    let path = format!("{}:/usr/bin:/bin", bin.display());
    let (out, _, code) = run_wib(&["init", "zsh"], &[("PATH", &path)]);
    assert_eq!(code, 0);
    assert_eq!(out.lines().next().unwrap(), format!("typeset -g _wib_bin='{}/wib'", bin.display()));
}

fn has_zsh() -> bool {
    Command::new("zsh").args(["-f", "-c", "true"]).status().is_ok_and(|s| s.success())
}

#[test]
fn init_with_a_missing_binary_says_so_and_the_shell_still_starts() {
    if !has_zsh() {
        return;
    }
    let sb = Sandbox::new("init-missing", &[]);
    let (out, _, _) = run_wib(&["init", "zsh"], &[]);
    let script = out.split_once('\n').unwrap().1;
    let file = sb.data().join("init.zsh");
    fs::write(&file, format!("typeset -g _wib_bin='/nonexistent/wib'\n{script}")).unwrap();
    let out = Command::new("zsh")
        .args(["-f", "-i", "-c", &format!("source '{}'; print ok", file.display())])
        .env("HOME", sb.data())
        .output()
        .unwrap();
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(stderr.contains("wib: cannot run /nonexistent/wib"), "{stderr}");
    assert!(stdout.contains("ok"), "{stdout}");
}

/// `wib setup zsh` with HOME (and optionally ZDOTDIR) inside the sandbox.
fn setup(sb: &Sandbox, path: &str, zdotdir: Option<&Path>) -> (String, String, i32) {
    let home = sb.data().join("home");
    fs::create_dir_all(&home).unwrap();
    let home = home.to_string_lossy().into_owned();
    let zdot = zdotdir.map(|z| z.to_string_lossy().into_owned());
    let mut env = vec![("HOME", home.as_str()), ("PATH", path)];
    if let Some(z) = &zdot {
        env.push(("ZDOTDIR", z));
    }
    run_wib(&["setup", "zsh"], &env)
}

#[test]
fn setup_appends_once_and_keeps_what_was_there() {
    let sb = Sandbox::new("setup-twice", &[]);
    let path = format!("{}:/usr/bin:/bin", wib_on_path(&sb).display());
    let rc = sb.data().join("home/.zshrc");
    fs::create_dir_all(rc.parent().unwrap()).unwrap();
    fs::write(&rc, "alias ll='ls -l'").unwrap();

    let (out, _, code) = setup(&sb, &path, None);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("added wib to ~/.zshrc (line 4)") && out.contains("exec zsh"), "{out}");
    let text = fs::read_to_string(&rc).unwrap();
    assert!(text.starts_with("alias ll='ls -l'\n\n# >>> wib >>>\n"), "{text}");
    assert!(!text.contains("export PATH"), "{text}");

    let (out, _, code) = setup(&sb, &path, None);
    assert_eq!(code, 0);
    assert!(out.contains("already set up: ~/.zshrc:4"), "{out}");
    assert_eq!(fs::read_to_string(&rc).unwrap(), text);
    assert_eq!(text.matches("wib init zsh").count(), 1);
}

#[test]
fn setup_adds_path_when_wib_is_not_on_it() {
    let sb = Sandbox::new("setup-path", &[]);
    let (out, _, code) = setup(&sb, "/usr/bin:/bin", None);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("(line 3)"), "{out}");
    let text = fs::read_to_string(sb.data().join("home/.zshrc")).unwrap();
    let dir = Path::new(env!("CARGO_BIN_EXE_wib")).parent().unwrap().display().to_string();
    let path_line = format!("export PATH=\"{dir}:$PATH\"");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines, ["# >>> wib >>>", path_line.as_str(), "eval \"$(wib init zsh)\"", "# <<< wib <<<"]);
}

#[test]
fn setup_writes_to_zdotdir_and_through_symlinks() {
    let sb = Sandbox::new("setup-zdot", &[]);
    let path = format!("{}:/usr/bin:/bin", wib_on_path(&sb).display());
    let zdot = sb.data().join("zdot");
    fs::create_dir_all(&zdot).unwrap();
    let (out, _, code) = setup(&sb, &path, Some(&zdot));
    assert_eq!(code, 0, "{out}");
    assert!(fs::read_to_string(zdot.join(".zshrc")).unwrap().contains("wib init zsh"));
    assert!(!sb.data().join("home/.zshrc").exists());

    let dotfiles = sb.data().join("dotfiles");
    fs::create_dir_all(&dotfiles).unwrap();
    fs::write(dotfiles.join("zshrc"), "ls\n").unwrap();
    let rc = sb.data().join("home/.zshrc");
    std::os::unix::fs::symlink(dotfiles.join("zshrc"), &rc).unwrap();
    let (out, _, code) = setup(&sb, &path, None);
    assert_eq!(code, 0, "{out}");
    assert!(fs::symlink_metadata(&rc).unwrap().file_type().is_symlink());
    assert!(fs::read_to_string(dotfiles.join("zshrc")).unwrap().starts_with("ls\n\n# >>> wib >>>"));
}

#[test]
fn setup_refuses_what_it_should_not_write() {
    let sb = Sandbox::new("setup-refuse", &[]);
    let rc = sb.data().join("home/.zshrc");
    fs::create_dir_all(&rc).unwrap();
    let (out, err, code) = setup(&sb, "/usr/bin:/bin", None);
    assert_eq!(code, 2);
    assert!(err.contains("eval \"$(wib init zsh)\"") && out.is_empty(), "{err}");
    assert!(fs::read_dir(&rc).unwrap().next().is_none());

    fs::remove_dir(&rc).unwrap();
    fs::write(&rc, "# mine\neval \"$(wib init zsh)\"\n").unwrap();
    let (out, _, code) = setup(&sb, "/usr/bin:/bin", None);
    assert_eq!(code, 0);
    assert!(out.contains("already set up: ~/.zshrc:2"), "{out}");

    assert_eq!(run_wib(&["setup"], &[]).2, 2);
    assert_eq!(run_wib(&["setup", "bash"], &[]).2, 2);
}

#[test]
fn setup_leaves_another_wib_on_path_in_charge() {
    let sb = Sandbox::new("setup-other", &[]);
    let other = sb.data().join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("wib"), "#!/bin/sh\n").unwrap();
    fs::set_permissions(other.join("wib"), fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:/usr/bin:/bin", other.display());
    let (out, _, code) = setup(&sb, &path, None);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(&format!("note: new shells will load {}/wib", other.display())), "{out}");
    assert!(!fs::read_to_string(sb.data().join("home/.zshrc")).unwrap().contains("export PATH"));
}

#[test]
fn setup_finds_zdotdir_set_in_zshenv_without_export() {
    if !has_zsh() {
        return;
    }
    let sb = Sandbox::new("setup-zshenv", &[]);
    let path = format!("{}:/usr/bin:/bin", wib_on_path(&sb).display());
    let zdot = sb.data().join("zdot");
    fs::create_dir_all(&zdot).unwrap();
    fs::create_dir_all(sb.data().join("home")).unwrap();
    fs::write(sb.data().join("home/.zshenv"), format!("ZDOTDIR='{}'\n", zdot.display())).unwrap();
    let (out, _, code) = setup(&sb, &path, None);
    assert_eq!(code, 0, "{out}");
    assert!(fs::read_to_string(zdot.join(".zshrc")).unwrap().contains("wib init zsh"));
    assert!(!sb.data().join("home/.zshrc").exists());
}

#[test]
fn setup_refusal_includes_the_path_line_it_would_add() {
    let sb = Sandbox::new("setup-refuse-path", &[]);
    fs::create_dir_all(sb.data().join("home/.zshrc")).unwrap();
    let (_, err, code) = setup(&sb, "/usr/bin:/bin", None);
    assert_eq!(code, 2);
    assert!(err.contains("export PATH=") && err.contains("eval \"$(wib init zsh)\""), "{err}");
}

#[test]
fn setup_already_done_succeeds_from_a_directory_it_could_not_quote() {
    let sb = Sandbox::new("setup-odd-dir", &[]);
    let odd = sb.data().join("we$ird");
    fs::create_dir_all(&odd).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_wib"), odd.join("wib")).unwrap();
    let home = sb.data().join("home");
    fs::create_dir_all(&home).unwrap();
    let rc = home.join(".zshrc");
    let run = || {
        let out = Command::new(odd.join("wib"))
            .args(["setup", "zsh"])
            .env_remove("ZDOTDIR")
            .env("HOME", &home)
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        (String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code())
    };
    fs::write(&rc, "ls\n").unwrap();
    let (_, err, code) = run();
    assert_eq!(code, Some(2));
    assert!(err.contains("need quoting by hand") && err.contains("<dir of wib>"), "{err}");
    assert_eq!(fs::read_to_string(&rc).unwrap(), "ls\n");

    fs::write(&rc, "eval \"$(wib init zsh)\"\n").unwrap();
    let (out, err, code) = run();
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("already set up: ~/.zshrc:1"), "{out}");
}

#[test]
fn setup_names_a_missing_zdotdir() {
    let sb = Sandbox::new("setup-no-zdot", &[]);
    let path = format!("{}:/usr/bin:/bin", wib_on_path(&sb).display());
    let zdot = sb.data().join("missing");
    let (_, err, code) = setup(&sb, &path, Some(&zdot));
    assert_eq!(code, 2);
    assert!(err.contains("neither does its directory") && err.contains("ZDOTDIR"), "{err}");
    assert!(!zdot.exists());
}

#[test]
fn doctor_reads_a_zshrc_that_is_not_utf8_and_names_an_unreadable_one() {
    let sb = Sandbox::new("doctor-rc-bytes", &[]);
    let home = sb.data().join("home");
    fs::create_dir_all(&home).unwrap();
    let rc = home.join(".zshrc");
    let home_s = home.to_string_lossy().into_owned();
    let hist = sb.data().join("zsh_history").to_string_lossy().into_owned();
    let data = sb.data().join("data").to_string_lossy().into_owned();
    let zdot = format!("ZDOTDIR={home_s}\nzsh_version=5.9");
    let doctor_in = |state: Option<&str>| {
        let mut env = vec![("WIB_ZSH_HISTORY", hist.as_str()), ("WIB_DATA_DIR", &data), ("HOME", &home_s)];
        if let Some(s) = state {
            env.push(("_WIB_DOCTOR_STATE", s));
        }
        run_wib(&["doctor"], &env).0
    };
    let doctor = || doctor_in(None);

    fs::write(&rc, b"# caf\xe9\neval \"$(wib init zsh)\"\n").unwrap();
    let out = doctor();
    assert!(out.contains("~/.zshrc:2 loads wib"), "{out}");

    fs::remove_file(&rc).unwrap();
    fs::create_dir(&rc).unwrap();
    let out = doctor();
    assert!(out.contains("cannot read ~/.zshrc") && out.contains("a regular file zsh can read"), "{out}");
    assert!(!out.contains("chmod u+r") && !out.contains("wib setup zsh"), "{out}");
    let out = doctor_in(Some(&zdot));
    assert!(out.contains("wib is loaded in this shell") && out.contains("cannot read ~/.zshrc"), "{out}");
    fs::remove_dir(&rc).unwrap();

    fs::write(&rc, "eval \"$(wib init zsh)\"\n").unwrap();
    fs::set_permissions(&rc, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&rc).is_ok() {
        return; // running as root: permissions do not stop the read
    }
    let out = doctor();
    fs::set_permissions(&rc, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(out.contains("cannot read ~/.zshrc") && out.contains("chmod u+r"), "{out}");
    assert!(!out.contains("wib setup zsh"), "{out}");
}
