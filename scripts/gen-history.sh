#!/usr/bin/env zsh
# Write a synthetic EXTENDED_HISTORY file: N commands spread evenly over the last
# DAYS days, oldest first like zsh writes them. A fixed seed makes runs repeatable.
# Usage: zsh scripts/gen-history.sh <count> <days> <out-file>
emulate -L zsh

integer count=${1:?usage: gen-history.sh <count> <days> <out-file>}
integer days=${2:?usage: gen-history.sh <count> <days> <out-file>}
out=${3:?usage: gen-history.sh <count> <days> <out-file>}

awk -v n=$count -v span=$(( days * 86400 )) -v now=$(date +%s) 'BEGIN {
  srand(42)
  nw = split("git status push origin master docker compose up down build cargo test run kubectl get pods logs npm install ls cd make", w, " ")
  for (i = 0; i < n; i++) {
    ts = now - span + int(i * span / n)
    k = int(rand() * 5) + 2
    cmd = w[int(rand() * nw) + 1]
    for (j = 1; j < k; j++) cmd = cmd " " w[int(rand() * nw) + 1]
    printf ": %d:0;%s %d\n", ts, cmd, i % 3000
  }
}' > $out
