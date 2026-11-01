#!/usr/bin/env zsh
# Measure `wib query` latency (including process spawn, as the zsh widget pays it)
# against a copy of a real history file. The original file is never modified.
# Usage: zsh scripts/bench.sh /path/to/wib [history-file] [runs] [prefix|keywords]
# `keywords` measures `wib query --keywords` with 1-2 words (3+ chars) of real commands.
emulate -L zsh
zmodload zsh/datetime

wib=${1:?usage: bench.sh /path/to/wib [history-file] [runs] [prefix|keywords]}
src=${2:-$HOME/.zsh_history}
integer runs=${3:-500}
mode=${4:-prefix}
[[ $mode == (prefix|keywords) ]] || { print -u2 "mode must be prefix or keywords"; exit 2 }

tmp=$(mktemp -d)
trap 'rm -rf $tmp' EXIT
cp "$src" $tmp/zsh_history
mkdir $tmp/data
export WIB_ZSH_HISTORY=$tmp/zsh_history WIB_DATA_DIR=$tmp/data

# Inputs: 2-5 char prefixes of real commands, so most queries return results.
local -a cmds inputs
cmds=( ${(f)"$(LC_ALL=C sed -n 's/^: [0-9]*:[0-9]*;//p' $tmp/zsh_history | LC_ALL=C grep -a '^[a-z]' | head -2000)"} )
(( ${#cmds} )) || { print -u2 "no commands found in $src"; exit 1 }
integer i len
local -a words
for (( i = 1; i <= runs; i++ )); do
  if [[ $mode == keywords ]]; then
    words=( ${(M)${=cmds[RANDOM % ${#cmds} + 1]}:#???*} )
    (( ${#words} )) || words=( ${=cmds[RANDOM % ${#cmds} + 1]} )
    len=$(( RANDOM % 2 + 1 ))
    inputs+=( "${(j:,:)${(@)words[1,len]}}" )
  else
    len=$(( RANDOM % 4 + 2 ))
    inputs+=( "${${cmds[RANDOM % ${#cmds} + 1]}[1,len]}" )
  fi
done
local -a flags=()
[[ $mode == keywords ]] && flags=( --keywords )

local -a ms
float t0
for input in $inputs; do
  t0=$EPOCHREALTIME
  "$wib" query -0 --period 30d --limit 8 "${flags[@]}" -- "$input" >/dev/null
  ms+=( $(( (EPOCHREALTIME - t0) * 1000 )) )
done

ms=( ${(on)ms} )
pct() { printf '%.2f' ${ms[$(( (${#ms} * $1 + 99) / 100 ))]} }
print "entries in file: $(LC_ALL=C grep -ac '^: [0-9]' $tmp/zsh_history)  runs: $runs  mode: $mode"
print "p50: $(pct 50) ms   p95: $(pct 95) ms   max: $(printf '%.2f' ${ms[-1]}) ms"
