<div align="center">

# wib — what is that bash?

**Your shell history, surfaced the moment you start typing.**

Stop digging through Ctrl-R. Stop retyping that 80-character `docker run`.
Type two letters and the command you ran last Tuesday is already under your cursor.

![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)
![zsh](https://img.shields.io/badge/shell-zsh%20%E2%89%A5%205.3-blue)
![No daemon](https://img.shields.io/badge/daemon-none-success)
![Local only](https://img.shields.io/badge/data-100%25%20local-success)
![License: MIT](https://img.shields.io/badge/license-MIT-lightgrey)

</div>

```text
PROMPT> gi
› git push origin master
  git status
  git commit -m "fix bug"
```

---

## The 30-second pitch

You already know the command. You ran it yesterday. You just can't remember whether it was
`--force-with-lease` or `-f`, which port you mapped, or the exact path to that config file.

Today you either press Ctrl-R and play guess-the-substring, or you scroll up through hundreds of
lines with ↑. **wib shows you the answer before you ask.** Type `gi` and you see your recent `git` commands.
Type `dcup` and you get `docker compose up`. Press ↓ and Enter and it runs.

- ⚡ **Instant**: about 5–8 ms per lookup on ~10k commands, including process startup.
- 🧠 **Smart matching**: prefix first, then word-initial fuzzy (`gi st` → `git status`).
- 🎯 **Ranked by you**: frequent and recent commands rise to the top, and duplicates collapse to one line.
- 🪶 **Featherweight**: one small Rust binary with zero dependencies, no daemon, no database server.
- 🔒 **Private by design**: no cloud, no telemetry, the log is owner-only (0600), and it respects `HISTORY_IGNORE`.
- 🤝 **Plays nice**: works with oh-my-zsh, powerlevel10k, zsh-autosuggestions and vi mode.
- 🔁 **Works on day one**: your existing `~/.zsh_history` powers suggestions immediately.

```sh
git clone https://github.com/duylongpro99/wuts-that-bash.git && cd wuts-that-bash
cargo install --path . && echo 'eval "$(wib init zsh)"' >> ~/.zshrc
```

Open a new terminal. That's it.

---

## How it matches

- Prefix matches come first, then fuzzy matching on word starts (`dcup` → `docker compose up`,
  `gi st` → `git status`). Each chunk you type must be the start of a word in the command; letters
  scattered across the middle of words don't count.
- Results are ranked by how often and how recently you used them; duplicate commands appear once.
- Suggestions come from the last 30 days by default (see `WIB_PERIOD`).
- Your data never leaves your machine.

## Why wib

- **Suggestions appear on their own.** Ctrl-R, fzf and atuin make you realize you need to search,
  then press a key to open a search screen. wib shows the list as you type and refines it with every keystroke.
- **See many options at once.** zsh-autosuggestions offers a single grey hint; wib shows up to 8 lines
  (configurable with `WIB_LIMIT`), so you recognize the right command when you see it, even if you
  couldn't recall how to spell it.
- **Type less, still land on the right command.** Word-initial matching (`dcup`, `gi st`) plus
  Shift+Tab word-by-word fill lets you reuse an old command as a template and edit only the end.
- **Doesn't break your habits.** ↑, Tab, Enter and Ctrl-R behave exactly as before; keys only change
  meaning after you press ↓ to select a line. Runs alongside oh-my-zsh, powerlevel10k,
  zsh-autosuggestions and vi mode. Your existing `~/.zsh_history` works right away.
- **Fast, light, private.** One binary, no daemon, no database server, no cloud, no telemetry.
  The log is mode 0600 and skips commands matching `HISTORY_IGNORE`. Uninstalling takes one line and one directory.

| | How you invoke it | Suggestions | Multi-machine sync |
|---|---|---|---|
| **wib** | Appears as you type | Several lines, right under the prompt | No, local only |
| zsh-autosuggestions | Appears as you type | One grey hint | No |
| Ctrl-R / fzf | Press a key to open | Search list | No |
| atuin | Press a key to open | Search list | Yes |

## Installation

Requires zsh ≥ 5.3 and Rust ≥ 1.89.

```sh
cargo install --path .
```

Add this to the **end** of `~/.zshrc` (after oh-my-zsh and other plugins):

```zsh
eval "$(wib init zsh)"
```

Open a new terminal and you're done. Your existing `~/.zsh_history` takes effect from the start.

## Key bindings

The list has two modes: **typing** (list visible, no line selected) and **selecting** (one line highlighted).

| Key | While typing | While selecting |
|-----|--------------|-----------------|
| ↓ | Start selecting, from the first line | Move down (wraps to the top at the end) |
| ↑ | zsh history, as usual | Move up (from the first line, back to typing) |
| Tab | zsh tab completion, as usual | Fill in the whole command without running it |
| Shift+Tab | Fill the first suggestion word by word | Fill the selected line word by word |
| Space | Type a space | Back to typing, no space added, list stays visible |
| Enter | Run what you typed | Fill in the selected command and run it |
| Esc / Ctrl-G | Hide the list until you type again | Same as left |

**Word-by-word fill:** type `gi`, select `git push origin master`, and press Shift+Tab four times
to get `git` → `git push` → `git push origin` → `git push origin master`. A quoted string like
`"fix bug"` is filled in a single press.

Cmd+Tab can't be used because macOS reserves it for app switching, so word-by-word fill uses Shift+Tab.

When the list isn't showing (too few characters, hidden, or while browsing history with ↑), every key
keeps its original behavior, including keys bound by oh-my-zsh (for example Shift+Tab stepping back in
the completion menu). ↑, Tab, Space and Enter only change meaning while a line is selected, so Ctrl-R,
completion and history are unaffected.

**Esc feels slow?** After Esc, zsh waits `KEYTIMEOUT` (0.4s by default) to tell it apart from arrow keys.
Set `KEYTIMEOUT=1` for an instant Esc. The trade-off is that pressing Esc then a key (as a substitute for
Option/Meta) stops working. Ctrl-G always hides the list immediately.

**vi mode** (`bindkey -v`): Esc hides the list and switches to command mode as usual.

## Keyword search

When you only remember a few words from an old command, type `wib` followed by those words. Keywords are
separated by commas or spaces, in any order, and a command must contain all of them.

```text
PROMPT> wib pull,master          PROMPT> wib pull
› git pull origin master         › git pull origin master
                                   docker pull nginx
```

- `wib pull master`, `wib master,pull` and `wib pull,master` give the same results.
- A keyword matches anywhere in the command (`8080` matches `docker run -p8080:80 nginx`). Commands where
  the keyword is a whole word rank first, then matches at the start of a word, then matches inside a word;
  ties are broken by frequency and recency.
- `wib` is the tool name, not a keyword. Previous `wib ...` invocations don't show up in results.
- The list appears once the text after `wib ` reaches `WIB_MIN_CHARS` characters. ↓ then Enter replaces
  the whole line with the selected command and runs it; Tab fills the command; Shift+Tab fills it word by word from the start.
- Pressing Enter with no line selected runs `wib pull,master` as a command and prints the list to the terminal.

## Configuration

Set environment variables in `~/.zshrc`:

| Variable | Default | Meaning |
|----------|---------|---------|
| `WIB_PERIOD` | `30d` | Time window to draw commands from: `12h`, `7d`, `2w`, `30d`, … |
| `WIB_LIMIT` | `8` | Maximum number of lines in the list |
| `WIB_MIN_CHARS` | `2` | Minimum characters before the list appears |
| `WIB_ZSH_HISTORY` | `$HISTFILE` | zsh history file |
| `WIB_DATA_DIR` | `~/.local/share/wib` | Where wib keeps its own log |

## Where your data lives

- wib reads `~/.zsh_history` (it needs `setopt EXTENDED_HISTORY` for timestamps; oh-my-zsh enables it by default).
- Every command you run is appended to `~/.local/share/wib/history.tsv` along with its directory and exit code.
  This log isn't capped by `SAVEHIST`, but it only keeps commands within `WIB_PERIOD`: once the oldest entry
  is more than `WIB_PERIOD` plus one day old, wib drops entries older than `WIB_PERIOD` (roughly once a day).
  Increasing `WIB_PERIOD` later therefore can't bring back commands already dropped from this log; older
  commands remain only in `~/.zsh_history` if `SAVEHIST` is large enough, and without directory or exit code.
- wib never modifies `~/.zsh_history`.
- Commands starting with a space and commands matching `HISTORY_IGNORE` are not recorded. The log file is
  readable only by its owner (mode 0600).
- **Limitation:** commands rejected by a custom `zshaddhistory` function are still recorded by wib, because
  zsh doesn't report which commands that function rejected. If you filter sensitive commands this way,
  switch to `HISTORY_IGNORE`.

## Speed and memory

**Why it's fast**

- zsh calls `wib query` once each time the buffer changes. Retyping the same content or moving through
  the list doesn't trigger another call.
- wib reads the history file directly and returns results, with no startup phase and no index to load.
  The binary is built with LTO and has no external library dependencies.
- wib reads `~/.zsh_history` backwards from the end and stops at the first command older than `WIB_PERIOD`,
  so the old part of the file is never read. `history.tsv` prunes itself by `WIB_PERIOD`. Lookup time
  therefore depends on how many commands fall inside `WIB_PERIOD`, not on how many years you've used your shell.
- Matching tries a prefix comparison first, since that's the common case. Fuzzy matching only runs when the
  input's letters appear in order in the command, so most commands are ruled out in a single scan. Scoring
  cost is proportional to input length times command length, so very long commands don't slow it down.
- Recording a command to the log (`wib record`) runs in the background after the command finishes, so it
  never delays your prompt.

**Why it doesn't leak memory**

wib has no daemon and no long-running process. Each lookup is its own process that exits when done, and
the OS reclaims all of its memory. Keeping a terminal open for days or typing thousands of keystrokes
won't make memory grow. The zsh shell itself holds at most `WIB_LIMIT` suggestion lines from the latest lookup.

**Benchmarks** (`scripts/bench.sh`, macOS, `WIB_PERIOD=30d`, process startup included, 100 lookups per
file; synthetic history files generated by `scripts/gen-history.sh`):

| History file | Commands within 30 days | p95 | Peak memory per lookup |
|---|---|---|---|
| 10,000 commands over 400 days | ~750 | 5.4 ms | ~2 MB |
| 100,000 commands over 400 days | ~7,500 | 8.8 ms | ~3.5 MB |
| 500,000 commands over 7 years (200/day) | ~6,000 | 6.9 ms | ~3.3 MB |
| 500,000 commands over 400 days (1,250/day) | ~37,500 | 22 ms | ~12 MB |

The last three rows show lookup time tracks the number of commands in the period: a 500,000-command
file spanning 7 years is faster than a 100,000-command file spanning 400 days. You'd only notice any lag
if you type tens of thousands of commands within one period; in that case, lower `WIB_PERIOD`.

Keyword mode (`bench.sh … keywords`) has a p95 comparable to normal mode: 7–10 ms on the
100,000-command, 400-day file.

## Using it with other plugins

- **zsh-autosuggestions:** the grey hint still appears; pressing → accepts only that hint, and wib's list is never pasted into the command line.
- **powerlevel10k / oh-my-zsh:** put the `eval` line after all plugins have loaded so wib captures the original key bindings.

## Known limitations

- zsh only; bash and fish are not supported yet.
- Installation requires a Rust toolchain; there's no Homebrew formula or prebuilt binary yet.
- No history sync between machines.
- wib assumes `~/.zsh_history` is written in chronological order (with up to one day of skew allowed, such as
  when `sharehistory` writes a long-running command after newer ones). If you append a history file from
  another machine to the end, newer commands located before the appended block may not be suggested.
- Pressing Ctrl-C while the list is visible leaves the list in the scrollback (like zsh's completion list).
- Right after recalling a command with ↑/↓ or Ctrl-R, the list stays hidden so ↓ keeps browsing history;
  type another character and the list reappears.

## Command-line usage

```sh
wib pull,master                   # keyword search, any order
wib -- init                       # search for text that collides with a subcommand (init, query, record)
wib query --period 7d -- dcup     # print matching commands, one per line
wib query --keywords -- pull,master
wib query -0 -- git               # NUL-separated output, preserves multi-line commands
```

`wib <keywords>` takes its period and line count from `WIB_PERIOD` and `WIB_LIMIT`, and reads history from
`WIB_ZSH_HISTORY` or `~/.zsh_history`. Variables set in `~/.zshrc` without `export` aren't visible to this
command, so if you change those values (or use a different `HISTFILE`), `export` them, for example
`export WIB_ZSH_HISTORY=$HISTFILE`.

## Uninstall

Remove the `eval "$(wib init zsh)"` line from `~/.zshrc`, then:

```sh
cargo uninstall wib
rm -rf ~/.local/share/wib
```

## Development

```sh
cargo test                                   # unit + CLI tests
cargo install --path . --root .bin
zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"    # drives a real zsh via zpty: keys, Ctrl-R, vi mode, autosuggestions
zsh scripts/bench.sh "$PWD/.bin/bin/wib"     # measure p50/p95 on a copy of ~/.zsh_history
zsh scripts/gen-history.sh 500000 400 /tmp/h  # generate a synthetic history: 500k commands over 400 days
zsh scripts/bench.sh "$PWD/.bin/bin/wib" /tmp/h 100
zsh scripts/bench.sh "$PWD/.bin/bin/wib" /tmp/h 100 keywords   # benchmark keyword mode
```

---

<div align="center">

**If wib saved you a Ctrl-R today, give it a ⭐. It helps other terminal dwellers find it.**

</div>
