#!/usr/bin/env zsh
# Measure `wib query` latency (including process spawn, as the zsh widget pays it)
# against a copy of a real history file. The original file is never modified.
# Usage: zsh scripts/bench.sh /path/to/wib [history-file] [runs]
emulate -L zsh
zmodload zsh/datetime

wib=${1:?usage: bench.sh /path/to/wib [history-file] [runs]}
src=${2:-$HOME/.zsh_history}
integer runs=${3:-500}

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
for (( i = 1; i <= runs; i++ )); do
  len=$(( RANDOM % 4 + 2 ))
  inputs+=( "${${cmds[RANDOM % ${#cmds} + 1]}[1,len]}" )
done

local -a ms
float t0
for input in $inputs; do
  t0=$EPOCHREALTIME
  "$wib" query -0 --period 30d --limit 8 -- "$input" >/dev/null
  ms+=( $(( (EPOCHREALTIME - t0) * 1000 )) )
done

ms=( ${(on)ms} )
pct() { printf '%.2f' ${ms[$(( (${#ms} * $1 + 99) / 100 ))]} }
print "entries in file: $(LC_ALL=C grep -ac '^: [0-9]' $tmp/zsh_history)  runs: $runs"
print "p50: $(pct 50) ms   p95: $(pct 95) ms   max: $(printf '%.2f' ${ms[-1]}) ms"
