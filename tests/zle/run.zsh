#!/usr/bin/env zsh
# Drives interactive zsh shells through zpty with wib loaded and checks key behaviour,
# including coexistence with Ctrl-R, history browsing, vi mode and zsh-autosuggestions.
# Usage: zsh tests/zle/run.zsh /path/to/wib [path/to/zsh-autosuggestions.zsh]
emulate -L zsh
setopt extended_glob
zmodload zsh/zpty zsh/datetime

wib=${1:?usage: run.zsh /path/to/wib [zsh-autosuggestions.zsh]}
[[ -x $wib ]] || { print -u2 "not executable: $wib"; exit 2 }
autosuggest=${2:-$HOME/.oh-my-zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.plugin.zsh}

tmp=$(mktemp -d)
# zpty forks; a forked child running this trap on exit must not remove $tmp.
zmodload zsh/system
integer main_pid=$sysparams[pid]
trap '(( sysparams[pid] == main_pid )) && { zpty -d wt 2>/dev/null; rm -rf $tmp }' EXIT
integer now=$EPOCHSECONDS failures=0 passes=0

# ---------------------------------------------------------------- harness

# start_shell <extra .zshrc lines> [extra history commands...]: fresh history,
# fresh wib log, new interactive zsh.
start_shell() {
  zpty -d wt 2>/dev/null
  rm -rf $tmp/data
  local extra
  {
    for extra in "${@:2}"; print -r -- ": $(( now - 950 )):0;$extra"
    print -r -- ": $(( now - 900 )):0;git status"
    print -r -- ": $(( now - 800 )):0;git commit -m \"fix bug\""
    print -r -- ": $(( now - 700 )):0;docker compose up"
    print -r -- ": $(( now - 600 )):0;echo ran-\$((6*7))"
    print -r -- ": $(( now - 500 )):0;git push origin master"
    print -r -- ": $(( now - 400 )):0;git push origin master"
    print -r -- ": $(( now - 300 )):0;git push origin master"
  } > $tmp/zsh_history
  cat > $tmp/.zshrc <<EOF
PS1='PROMPT> '
HISTFILE=$tmp/zsh_history
HISTSIZE=1000 SAVEHIST=1000
setopt extended_history hist_ignore_space
HISTORY_IGNORE='(*TOKEN*)'
KEYTIMEOUT=1
export WIB_DATA_DIR=$tmp/data
$1
eval "\$('$wib' init zsh)"
_wib_test_probe() { zle -M "PROBE{{\${BUFFER}|\${_wib_state}|\${#_wib_items}}}" }
zle -N _wib_test_probe
for km in main vicmd; bindkey -M \$km '^X^P' _wib_test_probe
zle() { [[ \$1 == -M ]] && print -r -- "\$2" >> $tmp/messages; builtin zle "\$@" }
_wib_test_cur() { zle -M "CUR{{\$CURSOR|\$MARK|\$REGION_ACTIVE}}" }
zle -N _wib_test_cur
for km in main vicmd; bindkey -M \$km '^X^K' _wib_test_cur
_wib_test_hl() { zle -M "HL{{\${#region_highlight}|\$KEYMAP}}" }
zle -N _wib_test_hl
bindkey '^X^H' _wib_test_hl
EOF
  [[ -n ${WIB_TEST_NO_MEMO-} ]] && print -r -- '_wib_memo=' >> $tmp/.zshrc
  zpty wt env HOME=$tmp ZDOTDIR=$tmp TERM=xterm-256color COLUMNS=120 LINES=40 \
    zsh -o no_global_rcs -i
  sleep 1; drain
}

out=''
drain() {
  local chunk
  integer i
  out=''
  # zpty -r -t only returns what is ready right now; poll briefly to collect a full redraw.
  for i in {1..10}; do
    # One character at a time: reading a line could wait for a newline that never comes.
    while zpty -r -t wt chunk '?'; do out+=$chunk; done
    sleep 0.03
  done
  out=${out//$'\e'\[[0-9;?]#[a-zA-Z]/}
  out=${out//$'\e'[=>]/}
}
send() { zpty -w -n wt "$1"; sleep 0.3; drain }
more() { local before=$out; sleep 0.3; drain; out=$before$out }  # keep collecting after send
probe() {
  send $'\C-x\C-p'
  local p=${out##*PROBE\{\{}
  REPLY=${p%%\}\}*}
}
buffer() { probe; REPLY=${REPLY%%\|*} }
cursor() {  # REPLY = CURSOR|MARK|REGION_ACTIVE
  send $'\C-x\C-k'
  local p=${out##*CUR\{\{}
  REPLY=${p%%\}\}*}
}
check() {  # check <description> <expected> <actual>
  if [[ $2 == "$3" ]]; then
    (( passes++ )); print -r -- "ok   - $1"
  else
    (( failures++ )); print -r -- "FAIL - $1"$'\n'"       expected: $2"$'\n'"       actual:   $3"
  fi
}
reset_line() { send $'\C-u' }

# ---------------------------------------------------------------- emacs mode

start_shell ''

# AC7: one character shows no list.
send 'g'; probe
check "1 char shows no list" "g|typing|0" "$REPLY"

# AC1: two characters show suggestions, rendered under the prompt.
send 'i'; screen=$out; probe
check "2 chars fetch suggestions" "gi|typing|3" "$REPLY"
[[ $screen == *"git push origin master"* ]]; check "list is drawn below prompt" 0 $?

# AC2: Down selects, Tab fills the whole command without running it.
send $'\e[B'; probe
check "down enters navigation" "gi|navigating|3" "$REPLY"
send $'\e[B'; send $'\e[A'; probe
check "down/up move within the list" "gi|navigating|3" "$REPLY"
send $'\t'; probe
check "tab fills the selected command" "git push origin master|typing|${REPLY##*\|}" "$REPLY"
[[ $out != *"Everything up-to-date"* && $out != *"not a git repository"* ]]; check "tab does not run the command" 0 $?

# AC8: Space leaves navigation without inserting a space; list stays.
reset_line; send 'gi'; send $'\e[B'; send ' '; probe
check "space returns to typing, buffer unchanged" "gi|typing|3" "$REPLY"
send ' '; buffer
check "space while typing inserts a space" "gi " "$REPLY"

# AC9: Esc hides the list; typing again shows it.
reset_line; send 'gi'; send $'\e'; probe
check "esc hides the list" "gi|hidden|3" "$REPLY"
send 't'; probe
check "typing after esc shows the list again" "git|typing|3" "$REPLY"
reset_line; send 'gi'; send $'\C-g'; probe
check "ctrl-g hides the list" "gi|hidden|3" "$REPLY"
reset_line; send 'gi'; send $'\e[B'; send $'\e'; probe
check "esc while navigating hides the list" "gi|hidden|3" "$REPLY"

# AC10: Shift+Tab fills the selected command word by word.
reset_line; send 'gi'; send $'\e[B'
local e
for e in 'git' 'git push' 'git push origin' 'git push origin master'; do
  send $'\e[Z'; buffer
  check "shift+tab -> '$e'" "$e" "$REPLY"
done
send $'\e[Z'; buffer
check "shift+tab at the end changes nothing" "git push origin master" "$REPLY"

# AC11: a quoted argument is one word.
reset_line; send 'git c'; send $'\e[B'
send $'\e[Z'; send $'\e[Z'; send $'\e[Z'; buffer
check "quoted string filled in one step" 'git commit -m "fix bug"' "$REPLY"

# Fuzzy goal: first Shift+Tab replaces the input with the first word.
reset_line; send 'dcup'; send $'\e[Z'; buffer
check "fuzzy match fills first word" "docker" "$REPLY"

# AC2: Enter on a selected row fills and runs it.
reset_line; send 'ec'; send $'\e[B'; send $'\r'; more
[[ $out == *"ran-42"* ]]; check "enter runs the selected command" 0 $?

# AC4: a command just run is suggested at the next prompt.
send 'echo wib-marker-$((1+1))'; send $'\r'; more
send 'echo wib'; send $'\e[B'; send $'\t'; buffer
check "just-run command is suggested" 'echo wib-marker-$((1+1))' "$REPLY"

# Up arrow while typing keeps zsh history behaviour.
reset_line; send $'\e[A'; buffer
check "up arrow while typing = zsh history" 'echo wib-marker-$((1+1))' "$REPLY"

# Down while browsing history goes forward in history, never into the list.
reset_line; send $'\r'; more   # fresh line: Ctrl-U alone keeps the history position
send $'\e[A'; send $'\e[A'; send $'\e[B'; probe
check "up, up, down browses history" 'echo wib-marker-$((1+1))|typing|0' "$REPLY"
send $'\C-u'; send 'gi'; probe
check "typing after clearing a recalled line shows the list" "gi|typing|3" "$REPLY"

# Ctrl-R: a space in the search pattern stays part of the search.
reset_line; send $'\C-r'; send 'push or'; buffer
check "ctrl-r search with a space" "git push origin master" "$REPLY"
reset_line

# Ctrl-R started while navigating: afterwards keys behave as in main, Tab never wipes the line.
reset_line; send $'\r'; more
send 'gi'; send $'\e[B'; send $'\C-r'; send 'commit'; send $'\C-e'; send $'\t'; buffer
check "ctrl-r from navigation, then tab keeps the line" 'git commit -m "fix bug"' "${REPLY% }"
send ' '; buffer
check "ctrl-r from navigation, then space inserts" 'git commit -m "fix bug" ' "${REPLY/%  / }"
reset_line; send $'\r'; more

# Highlights do not pile up while editing, and no stale selection remains.
send 'git p'; send $'\x7f'; send $'\x7f'; send $'\e[B'; send $'\e[B'; send 't'
send $'\C-x\C-h'; hl=${${out##*HL\{\{}%%\}\}*}
probe; integer rows=${REPLY##*\|}
check "highlights match visible rows after edits" "$rows|main" "$hl"
reset_line

# Tab while typing keeps zsh completion.
reset_line; send 'ech'; send $'\t'; buffer
check "tab while typing = zsh completion" "echo" "$REPLY"

# Space-prefixed and HISTORY_IGNORE'd commands are not recorded; the log is private.
reset_line; send ' echo secret-$((2+3))'; send $'\r'; more
send 'echo MY_TOKEN=$((2+4))'; send $'\r'; more
sleep 0.3
grep -qE 'secret|TOKEN' $tmp/data/history.tsv 2>/dev/null
check "filtered commands are not recorded" 1 $?
check "log file is private" 600 "$(stat -f %Lp $tmp/data/history.tsv)"

# ---------------------------------------------------------------- secret guard

# The secret must not match the harness's HISTORY_IGNORE='(*TOKEN*)'.
secret=Zm9vYmFyYmF6cXV4cXV1eDEyMzQ1Njc4OTBhYmNk
secret_cmd="curl -s -H 'Authorization: Bearer $secret' localhost:1"
notice='wib: the last command looks like it contains a secret header'

# The binary sees the command only on stdin, never in its arguments.
print -r -- "#!/bin/sh
printf '%s\n' \"\$*\" >> $tmp/spy.log
exec '$wib' \"\$@\"" > $tmp/spy
chmod +x $tmp/spy
reset_line; send "_wib_bin=$tmp/spy"; send $'\r'; more
send "$secret_cmd"; send $'\r'; more; sleep 0.5; screen=$out
send 'x'; screen+=$out
[[ $screen == *"$notice"* ]]; check "secret: the shell is told at the next prompt" 0 $?
grep -q curl $tmp/data/history.tsv 2>/dev/null
check "secret: the command is not recorded" 1 $?
grep -q record $tmp/spy.log && ! grep -q "$secret" $tmp/spy.log
check "secret: never in the arguments of wib" 0 $?
reset_line; send 'cur'; probe
check "secret: a command not recorded is not suggested" "cur|typing|0" "$REPLY"
reset_line; send 'echo'; send $'\r'; more; sleep 0.3; send 'x'
[[ $out != *"$notice"* ]]; check "secret: the notice is shown once" 0 $?

reset_line; send 'echo MY_TOKEN=abcdef123456'; send $'\r'; more; sleep 0.5; screen=$out
send 'x'; screen+=$out
[[ $screen != *"looks like it contains"* ]]; check "secret: no notice for a HISTORY_IGNORE'd command" 0 $?
reset_line

# A secret only in .zsh_history is not suggested; another shell's notice is not shown.
start_shell '' "$secret_cmd" 'curl -s localhost:2'
send 'cur'; probe
check "secret: only in .zsh_history, still not suggested" "cur|typing|1" "$REPLY"
reset_line
zpty wt2 env HOME=$tmp ZDOTDIR=$tmp TERM=xterm-256color COLUMNS=120 LINES=40 zsh -o no_global_rcs -i
sleep 1
send "$secret_cmd"; send $'\r'; more; sleep 0.5
zpty -w -n wt2 'x'; sleep 0.5
other=''; while zpty -r -t wt2 chunk '?'; do other+=$chunk; done
[[ -n $other && $other != *"looks like it contains"* ]]; check "secret: another shell does not show the notice" 0 $?
zpty -d wt2

# ---------------------------------------------------------------- keyword search

start_shell "path=('${wib:h}' \$path); export WIB_ZSH_HISTORY=$tmp/zsh_history" 'echo docker pull nginx' 'echo pull origin master' 'wib pull,master'

send 'wib p'; probe
check "keywords: below WIB_MIN_CHARS shows no list" "wib p|typing|0" "$REPLY"
send 'ull'; probe
check "keywords: one keyword" "wib pull|typing|2" "$REPLY"
send ',master'; probe
check "keywords: two keywords, old wib calls skipped" "wib pull,master|typing|1" "$REPLY"
reset_line; send 'wib master pull'; probe
check "keywords: any order, space separated" "wib master pull|typing|1" "$REPLY"
send $'\e[B'; send $'\t'; buffer
check "keywords: tab replaces the whole line" "echo pull origin master" "$REPLY"
reset_line; send 'wib pull,master'; send $'\e[Z'; buffer
check "keywords: shift+tab fills the first word" "echo" "$REPLY"
reset_line; send 'wib pull,master'; send $'\e[B'; send $'\r'; more
sleep 0.3
[[ $(tail -n 1 $tmp/data/history.tsv 2>/dev/null) == *$'\t'"echo pull origin master" ]]
check "keywords: enter runs the selected command" 0 $?
send 'wib nginx'; send $'\r'; more
[[ $out == *"echo docker pull nginx"* ]]; check "keywords: enter without a selection prints matches" 0 $?
reset_line; send 'gi'; probe
check "keywords: normal typing unchanged" "gi|typing|3" "$REPLY"

# ---------------------------------------------------------------- project ranking and forget

mkdir -p $tmp/r1/.git $tmp/r2/.git
start_shell "path=('${wib:h}' \$path); export WIB_ZSH_HISTORY=$tmp/zsh_history"
send "cd $tmp/r1"; send $'\r'; more
send 'echo mark-r1'; send $'\r'; more
send "cd $tmp/r2"; send $'\r'; more
send 'echo mark-r2'; send $'\r'; more
send "cd $tmp/r1"; send $'\r'; more
sleep 0.3
send 'echo mark'; send $'\e[B'; send $'\t'; buffer
check "project: this repo's command comes first" "echo mark-r1" "$REPLY"
reset_line; send "cd $tmp/r2"; send $'\r'; more
send 'echo mark'; send $'\e[B'; send $'\t'; buffer
check "project: changing repo changes the first row" "echo mark-r2" "$REPLY"
reset_line; send 'wib forget echo mark-r2'; send $'\r'; more
sleep 0.3
send 'echo mark'; probe
check "forget: the command leaves the list" "echo mark|typing|1" "$REPLY"
reset_line; send 'wib forget gi'; probe
check "forget: wib forget is not a keyword search" "wib forget gi|typing|0" "$REPLY"
reset_line; send 'wib unforget echo mark-r2'; send $'\r'; more
sleep 0.3
send 'echo mark'; probe
check "unforget: the command is back in the list" "echo mark|typing|2" "$REPLY"
reset_line; send 'wib doctor'; send $'\r'; more; sleep 1; more
[[ $out == *"ok    Down, Shift+Tab, Esc, Ctrl-G and Ctrl-] are bound to wib"* ]]
check "doctor: reports the keys bound to wib" 0 $?
[[ $out != *$'\n'"fail "* && $out != *$'\r'"fail "* ]]; check "doctor: no failed check" 0 $?
reset_line

# ---------------------------------------------------------------- template slots

runs=( 'docker run -p 8080:80 nginx:1.25' 'docker run -p 9090:80 nginx:1.24' )
start_shell '' "${runs[@]}"
send 'docker r'; send $'\e[B'; send $'\C-]'; buffer
check "slots: ctrl-] on a selected row fills it" "${runs[1]}" "$REPLY"
cursor
check "slots: cursor at the end of the port, port selected" "21|14|1" "$REPLY"
send '7070:80'; buffer
check "slots: typing replaces the whole port" "docker run -p 7070:80 nginx:1.25" "$REPLY"
send $'\C-]'; cursor
check "slots: ctrl-] again goes to the tag" "32|22|1" "$REPLY"
send $'\C-]'; cursor
check "slots: past the last slot, back to the first" "21|14|1" "$REPLY"
send $'\C-b'; send $'\C-f'; send '0'; buffer
check "slots: after moving the cursor, typing just inserts" "docker run -p 7070:800 nginx:1.25" "$REPLY"
send $'\C-]'; send $'\C-b'; send 'X'; buffer
check "slots: other edits do not replace the slot" "docker run -p 7070:800 nginx:1.2X5" "$REPLY"
# zpty may hold back a message line, so messages are read from the log the test shell keeps.
reset_line; send 'ls -la /tmp'; rm -f $tmp/messages; send $'\C-]'; buffer
check "slots: no earlier runs leaves the line alone" "ls -la /tmp" "$REPLY"
grep -q "wib: no earlier runs to compare" $tmp/messages; check "slots: no earlier runs is said" 0 $?
reset_line

# ---------------------------------------------------------------- did you mean

start_shell ''
send 'echo hi'; send $'\r'; more; sleep 0.3
send 'ehco hi'; send $'\r'; more; sleep 0.3; screen=$out; probe
check "fix: after exit 127 the corrected command is listed" "|typing|1" "$REPLY"
screen+=$out
[[ $screen == *"echo hi  ← did you mean?"* ]]; check "fix: the row says did you mean" 0 $?
send $'\e[B'; send $'\r'; more
[[ $out == *$'\n'hi* || $out == *$'\r'hi* ]]; check "fix: down, enter runs it" 0 $?
send 'ehco hi'; send $'\r'; more; sleep 0.3
send 'x'; probe
check "fix: typing drops the correction" "x|typing|0" "$REPLY"
send $'\C-h'; probe
check "fix: it does not come back on an empty line" "|typing|0" "$REPLY"
# Any other failure starts no extra process.
print -r -- "#!/bin/sh
printf '%s\n' \"\$1\" >> $tmp/spy.log
exec '$wib' \"\$@\"" > $tmp/spy
chmod +x $tmp/spy; rm -f $tmp/spy.log
reset_line; send "_wib_bin=$tmp/spy"; send $'\r'; more
send 'grep nothing /dev/null'; send $'\r'; more; sleep 0.3; probe
check "fix: exit 1 gives no correction" "|typing|0" "$REPLY"
[[ $(grep -c suggest-fix $tmp/spy.log) == 0 ]]; check "fix: exit 1 runs no lookup" 0 $?
reset_line

# ---------------------------------------------------------------- vi mode

start_shell 'bindkey -v' "${runs[@]}"
send 'git s'; send $'\e'; send '0'; send 'x'; buffer
check "vi: esc with list visible still enters command mode" "it s" "$REPLY"
send $'\e'; send 'cc'; send 'gi'; send $'\e[B'; send $'\e'; send '0'; send 'x'; buffer
check "vi: esc while navigating also enters command mode" "i" "$REPLY"
send $'\e'; send 'cc'; send "${runs[1]}"; send $'\C-]'; cursor
check "vi: ctrl-] in insert mode jumps to a slot" "21|14|1" "$REPLY"
send $'\e'; send 'cc'; send 'ls -la /tmp'; send $'\C-]'; buffer
check "vi: ctrl-] without a slot inserts nothing" "ls -la /tmp" "$REPLY"

# ---------------------------------------------------------------- zsh-autosuggestions

if [[ -r $autosuggest ]]; then
  start_shell "source '$autosuggest'"
  send 'git p'; screen=$out
  [[ $screen == *"git push origin master"*$'\n'* ]]; check "autosuggest: list still drawn" 0 $?
  send $'\e[C'; buffer
  check "autosuggest: right arrow accepts only the suggestion" "git push origin master" "$REPLY"
  send $'\e[B'; send $'\t'; buffer
  check "autosuggest: wib list still usable" "git push origin master" "${REPLY%% }"
  reset_line; send "source '$autosuggest'"; send $'\r'; more   # plugin reloaded
  send 'git p'; send $'\e[C'; buffer
  check "autosuggest: still safe after plugin reload" "git push origin master" "$REPLY"
else
  print -r -- "skip - zsh-autosuggestions not found at $autosuggest"
fi

print -r -- "---- $passes passed, $failures failed"
(( failures == 0 ))
