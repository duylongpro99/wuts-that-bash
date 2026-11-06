# wib — zsh integration. Load with: eval "$(wib init zsh)"
# `_wib_bin` is set by `wib init zsh` to the absolute path of the binary.
#
# States: typing (list shown, nothing selected), navigating (a row is selected),
#         hidden (dismissed with Esc / Ctrl-G until the buffer changes).
#
# Only ↓, Shift+Tab, Esc, Ctrl-G and Ctrl-] are bound in the main keymap, and each
# falls back to its previous widget when the list is not in play. The other list keys
# (↑, Tab, Space, Enter) live in the `wib-nav` keymap, which is active only while
# a row is selected, so Ctrl-R, completion and history keys keep working.

[[ -o interactive ]] || return 0
[[ -n ${_wib_bin-} && -x $_wib_bin ]] || return 0
(( ${+_wib_loaded} )) && return 0
typeset -g _wib_loaded=1

zmodload zsh/datetime zsh/parameter
zmodload -F zsh/files b:zf_rm
autoload -Uz add-zsh-hook add-zle-hook-widget is-at-least
# zsh 5.9+ can tag highlights so wib removes exactly its own; older zsh falls
# back to removing highlights that start past the buffer (where wib's list is).
typeset -g _wib_memo=''
is-at-least 5.9 && _wib_memo=' memo=wib'

typeset -g  _wib_state=typing
typeset -gi _wib_sel=0
typeset -ga _wib_items=()
typeset -g  _wib_last_buffer=$'\0'
typeset -g  _wib_tail=''        # text wib appended to POSTDISPLAY
typeset -g  _wib_goal=''        # command being filled word by word (Shift+Tab)
typeset -g  _wib_goal_buffer='' # buffer right after the last word fill
typeset -g  _wib_cmd=''         # command captured in preexec, recorded in precmd
typeset -gi _wib_cmd_ts=0
typeset -gA _wib_orig=()        # raw key sequence -> main-keymap widget before wib
typeset -ga _wib_slot_pending=() # (start end buffer) after Ctrl-]: typing replaces the slot
typeset -g  _wib_fix=''         # corrected command after "command not found", for one prompt

# Run the wib binary with the shell's settings, exported or not, so every
# subcommand reads the same history file, data directory and period.
_wib_run() {
  WIB_ZSH_HISTORY=${WIB_ZSH_HISTORY:-${HISTFILE:-$HOME/.zsh_history}} \
  WIB_DATA_DIR=${WIB_DATA_DIR-} XDG_DATA_HOME=${XDG_DATA_HOME-} WIB_PERIOD=${WIB_PERIOD-} \
  WIB_SECRET_GUARD=${WIB_SECRET_GUARD-} \
    "$_wib_bin" "$@"
}

# Same rule as the binary: $WIB_DATA_DIR, else $XDG_DATA_HOME/wib, else ~/.local/share/wib.
_wib_data_dir() {
  REPLY=${WIB_DATA_DIR:-${XDG_DATA_HOME:+$XDG_DATA_HOME/wib}}
  REPLY=${REPLY:-$HOME/.local/share/wib}
}

# `wib record` leaves secret-notice.<pid> when this shell's last command looked
# like it held a secret and was not saved. Show it once.
_wib_secret_notice() {
  local REPLY label
  _wib_data_dir
  local file=$REPLY/secret-notice.$$
  [[ -f $file ]] || return 0
  label=$(<$file)
  zf_rm -f -- $file
  zle -M "wib: the last command looks like it contains a ${label:-secret}; wib did not save it. It is still in your zsh history: run 'wib audit'."
}

_wib_zshexit() {
  local REPLY
  _wib_data_dir
  zf_rm -f -- $REPLY/secret-notice.$$ 2>/dev/null
  return 0
}

# ---------------------------------------------------------------- rendering

# Remove only what wib added, so other plugins' POSTDISPLAY (e.g. zsh-autosuggestions)
# and highlights survive.
_wib_clear_display() {
  if [[ -n $_wib_tail && $POSTDISPLAY == *"$_wib_tail" ]]; then
    POSTDISPLAY=${POSTDISPLAY%"$_wib_tail"}
  fi
  _wib_tail=''
  if [[ -n $_wib_memo ]]; then
    region_highlight=( "${(@)region_highlight:#*memo=wib}" )
  else
    # zsh shifts offsets as the buffer is edited, so match by position.
    local h
    local -a keep=()
    for h in "${region_highlight[@]}"; do
      (( ${${h#P}%% *} > ${#BUFFER} )) || keep+=( "$h" )
    done
    region_highlight=( "${keep[@]}" )
  fi
}

_wib_render() {
  setopt localoptions no_ksh_arrays no_sh_word_split
  _wib_clear_display
  [[ $_wib_state == hidden ]] && return
  (( ${#_wib_items} )) || return
  local item line text='' note=''
  local -a hl=()
  [[ -n $_wib_fix && -z $BUFFER ]] && note='  ← did you mean?'
  integer i=0 start offset=$(( ${#BUFFER} + ${#POSTDISPLAY} ))
  integer width=$(( COLUMNS - 4 - ${#note} > 4 ? COLUMNS - 4 - ${#note} : 4 ))
  for item in "${_wib_items[@]}"; do
    (( i++ ))
    line=${item//$'\n'/ ⏎ }
    (( ${#line} > width )) && line="${line[1,width-1]}…"
    line+=$note
    if (( i == _wib_sel )); then line="› $line"; else line="  $line"; fi
    start=$(( offset + ${#text} + 1 ))
    hl+=( "$start $(( start + ${#line} )) $( (( i == _wib_sel )) && print standout || print fg=244 )$_wib_memo" )
    text+=$'\n'$line
  done
  _wib_tail=$text
  POSTDISPLAY+=$text
  region_highlight+=( "${hl[@]}" )
}

# `wib <keywords>` on the line: REPLY = the keywords (wib itself is not one).
# `wib audit|doctor|forget|unforget|query|record|slots|suggest-fix|init ...` runs a subcommand, so it is a plain line.
_wib_keywords() {
  setopt localoptions extended_glob
  [[ $BUFFER == wib[[:blank:]]* ]] || return 1
  REPLY=${BUFFER#wib[[:blank:]]##}
  [[ $REPLY != (audit|doctor|forget|unforget|query|record|slots|suggest-fix|init)(|[[:blank:]]*) ]]
}

_wib_should_fetch() {
  # A correction after "command not found" shows on the empty line.
  [[ -z $BUFFER && -n $_wib_fix ]] && return 0
  local typed=$BUFFER REPLY
  _wib_keywords && typed=$REPLY
  (( ${#typed} >= ${WIB_MIN_CHARS:-2} && CURSOR == ${#BUFFER} )) || return 1
  # The buffer was just replaced by history browsing/search (↑/↓, Ctrl-R, …):
  # leave ↓ to history. Typing afterwards (even on a recalled line) shows the list.
  [[ $LASTWIDGET == *(history|search)* ]] && return 1
  (( ${ISEARCHMATCH_ACTIVE:-0} )) && return 1    # inside Ctrl-R / Ctrl-S
  [[ $BUFFER != *$'\n'* ]]
}

_wib_fetch() {
  _wib_items=()
  if [[ -z $BUFFER && -n $_wib_fix ]]; then
    _wib_items=( "$_wib_fix" )
    return
  fi
  _wib_should_fetch || return
  local out REPLY input=$BUFFER
  local -a mode=()
  _wib_keywords && mode=( --keywords ) input=$REPLY
  # The input goes through the environment, which other users cannot read in
  # `ps`, and costs no extra process per keystroke.
  out=$(WIB_INPUT=$input _wib_run query -0 --input-env --cwd "$PWD" --period "${WIB_PERIOD:-30d}" \
        --limit "${WIB_LIMIT:-8}" "${mode[@]}" 2>/dev/null) || return
  _wib_items=( "${(@0)out}" )
  _wib_items=( "${(@)_wib_items:#}" )
}

_wib_leave_nav() {
  [[ $KEYMAP == wib-nav ]] && zle -K main
}

# After Ctrl-] selected a slot, text typed at its end replaces the whole slot.
# Any other change (moving, deleting, pasting over the region) just ends it.
_wib_slot_typed() {
  (( ${#_wib_slot_pending} )) || return 0
  local old=${_wib_slot_pending[3]}
  integer s=${_wib_slot_pending[1]} e=${_wib_slot_pending[2]} added=$(( ${#BUFFER} - ${#old} ))
  [[ $BUFFER == "$old" ]] && (( CURSOR == e )) && return 0
  _wib_slot_pending=()
  REGION_ACTIVE=0
  if (( added > 0 && CURSOR == e + added )) && [[ ${BUFFER[1,e]} == "${old[1,e]}" && ${BUFFER[e+added+1,-1]} == "${old[e+1,-1]}" ]]; then
    BUFFER=${old[1,s]}${BUFFER[e+1,e+added]}${old[e+1,-1]}
    CURSOR=$(( s + added ))
  fi
}

# Runs before every redraw: refetch only when the buffer actually changed.
_wib_pre_redraw() {
  setopt localoptions no_ksh_arrays no_sh_word_split
  _wib_slot_typed
  if [[ $BUFFER != "$_wib_last_buffer" ]]; then
    _wib_last_buffer=$BUFFER
    [[ -n $BUFFER ]] && _wib_fix=''   # typing anything drops the correction
    _wib_state=typing
    _wib_sel=0
    _wib_leave_nav
    _wib_fetch
  fi
  [[ $_wib_state == navigating ]] || _wib_leave_nav
  _wib_render
  _wib_secret_notice
}

_wib_line_init() {
  _wib_state=typing
  _wib_sel=0
  _wib_items=()
  _wib_goal=''
  _wib_goal_buffer=''
  _wib_last_buffer=$'\0'
  _wib_tail=''
  _wib_slot_pending=()
  _wib_leave_nav
  _wib_wrap_autosuggest
  # The prompt is drawn before line-init, so show a correction now, not on the first key.
  [[ -n $_wib_fix ]] && _wib_pre_redraw
  _wib_secret_notice
}

_wib_line_finish() {
  _wib_fix=''
  _wib_clear_display
  _wib_leave_nav
  zle -R
}

# zsh-autosuggestions accepts a suggestion with BUFFER+=$POSTDISPLAY. Strip
# wib's list first so it never ends up in the command line. Wrapped lazily
# because the plugin may be loaded after wib.
_wib_wrap_autosuggest() {
  local f
  for f in _zsh_autosuggest_accept _zsh_autosuggest_partial_accept _zsh_autosuggest_execute; do
    (( ${+functions[$f]} )) || continue
    [[ $functions[$f] == *_wib_clear_display* ]] && continue   # already wrapped
    functions[_wib_orig$f]=$functions[$f]
    functions[$f]="_wib_clear_display; _wib_orig$f \"\$@\""
  done
}

# ---------------------------------------------------------------- key widgets

_wib_call_orig() {
  local w=${_wib_orig[$KEYS]-}
  [[ -n $w && $w != undefined-key ]] && zle "$w"
  return 0
}

_wib_list_visible() {
  [[ $_wib_state != hidden ]] && (( ${#_wib_items} ))
}

_wib_fill() {
  BUFFER=$1
  CURSOR=${#BUFFER}
}

_wib_to_typing() {
  _wib_state=typing
  _wib_sel=0
  _wib_leave_nav
}

# ↓ in main: start navigating when the list is visible, else the old widget.
_wib_down() {
  setopt localoptions no_ksh_arrays no_sh_word_split
  if _wib_list_visible && _wib_should_fetch; then
    _wib_state=navigating
    _wib_sel=1
    _wib_make_nav_keymap
    zle -K wib-nav
    _wib_render
  else
    _wib_call_orig
  fi
}

# wib-nav is rebuilt from main each time navigation starts, so key bindings
# added after wib loaded (fzf, user bindings) still work while navigating.
_wib_make_nav_keymap() {
  bindkey -N wib-nav main
  bindkey -M wib-nav '^[[B' _wib_nav_down    '^[OB' _wib_nav_down
  bindkey -M wib-nav '^[[A' _wib_nav_up      '^[OA' _wib_nav_up
  bindkey -M wib-nav '^I'   _wib_nav_tab
  bindkey -M wib-nav '^[[Z' _wib_word
  bindkey -M wib-nav '^]'   _wib_nav_slot
  bindkey -M wib-nav ' '    _wib_nav_space
  bindkey -M wib-nav '^M'   _wib_nav_enter   '^J' _wib_nav_enter
  bindkey -M wib-nav '^['   _wib_nav_dismiss '^G' _wib_nav_dismiss
}

# wib-nav can outlive navigation (Ctrl-R restores the keymap it started in).
# Then hand the key back to main untouched.
_wib_nav_active() {
  [[ $_wib_state == navigating ]] && (( _wib_sel > 0 && _wib_sel <= ${#_wib_items} )) && return 0
  _wib_to_typing
  zle -U -- "$KEYS"
  return 1
}

# The widgets below are bound only in wib-nav (a row is selected).
_wib_nav_down() {
  _wib_nav_active || return 0
  _wib_sel=$(( _wib_sel % ${#_wib_items} + 1 ))
  _wib_render
}

_wib_nav_up() {
  _wib_nav_active || return 0
  if (( _wib_sel > 1 )); then
    (( _wib_sel-- ))
  else
    _wib_to_typing
  fi
  _wib_render
}

_wib_nav_tab() {
  setopt localoptions no_ksh_arrays
  _wib_nav_active || return 0
  local cmd=${_wib_items[_wib_sel]}
  _wib_to_typing
  _wib_fill "$cmd"
}

_wib_nav_enter() {
  setopt localoptions no_ksh_arrays
  _wib_nav_active || return 0
  local cmd=${_wib_items[_wib_sel]}
  _wib_to_typing
  _wib_fill "$cmd"
  _wib_clear_display
  zle .accept-line
}

_wib_nav_space() {
  _wib_nav_active || return 0
  _wib_to_typing
  _wib_render
}

# Esc / Ctrl-G also drop a slot selection, so vi command mode does not start
# in visual mode on it.
_wib_drop_slot() {
  (( ${#_wib_slot_pending} )) || return 0
  _wib_slot_pending=()
  REGION_ACTIVE=0
}

_wib_nav_dismiss() {
  _wib_nav_active || return 0
  _wib_drop_slot
  _wib_to_typing
  _wib_state=hidden
  _wib_render
  [[ $KEYS == $'\e' && ${_wib_orig[$KEYS]-} == vi-cmd-mode ]] && zle vi-cmd-mode
}

# Esc / Ctrl-G in main: hide a visible list. In vi insert mode Esc also
# switches to command mode as usual; with no list, the old widget runs.
_wib_dismiss() {
  _wib_drop_slot
  if _wib_list_visible; then
    _wib_state=hidden
    _wib_render
    [[ $KEYS == $'\e' && ${_wib_orig[$KEYS]-} == vi-cmd-mode ]] && zle vi-cmd-mode
  else
    _wib_call_orig
  fi
}

# Ctrl-]: select the next part of the line that earlier runs of the same
# command changed (start at or after the cursor, else the first). Each press
# asks again from the current line, so nothing goes stale.
_wib_slot() {
  setopt localoptions no_ksh_arrays no_sh_word_split
  local out s pick=''
  local -a slots=()
  if [[ $BUFFER != *$'\n'* && -n ${BUFFER//[[:space:]]/} ]]; then
    out=$(print -rn -- "$BUFFER" | _wib_run slots --stdin --period "${WIB_PERIOD:-30d}" 2>/dev/null)
    slots=( ${(f)out} )
  fi
  if (( ! ${#slots} )); then
    zle -M "wib: no earlier runs to compare"
    # In vi insert mode the old widget is self-insert: it would insert ^].
    local w=${_wib_orig[$KEYS]-}
    [[ -n $w && $w != (self-insert|undefined-key) ]] && zle "$w"
    return 0
  fi
  for s in "${slots[@]}"; do
    (( ${s%% *} >= CURSOR )) && { pick=$s; break }
  done
  pick=${pick:-${slots[1]}}
  MARK=${pick%% *}
  CURSOR=${pick##* }
  REGION_ACTIVE=1
  _wib_slot_pending=( $MARK $CURSOR "$BUFFER" )
}

# Ctrl-] on a selected row: fill it (like Tab), then jump to its first slot.
_wib_nav_slot() {
  setopt localoptions no_ksh_arrays
  _wib_nav_active || return 0
  local cmd=${_wib_items[_wib_sel]}
  _wib_to_typing
  _wib_fill "$cmd"
  CURSOR=0
  _wib_slot
}

# Offset in $1 where the first shell word ending after offset $2 ends.
# Words follow zsh's own tokenizer, so a quoted "fix bug" is one word.
_wib_word_end() {
  local goal=$1 tok rest before
  integer from=$2 pos=0 end
  for tok in "${(@z)goal}"; do
    rest=${goal[pos+1,-1]}
    before=${rest%%${(b)tok}*}
    (( ${#before} == ${#rest} )) && continue   # synthetic token (e.g. newline → ';')
    end=$(( pos + ${#before} + ${#tok} ))
    if (( end > from )); then
      REPLY=$end
      return
    fi
    pos=$end
  done
  REPLY=${#goal}
}

_wib_word() {
  setopt localoptions no_ksh_arrays no_sh_word_split
  local goal=''
  # Keep the locked command while the buffer is still exactly what we last filled.
  if [[ -n $_wib_goal && $BUFFER == "$_wib_goal_buffer" ]]; then
    goal=$_wib_goal
  elif [[ $LASTWIDGET != *complete* ]] && _wib_list_visible; then
    goal=${_wib_items[_wib_sel > 0 ? _wib_sel : 1]}
  fi
  if [[ -z $goal ]]; then
    _wib_call_orig
    return
  fi
  _wib_goal=$goal
  (( ${#BUFFER} >= ${#goal} )) && [[ ${(L)BUFFER} == ${(L)goal} ]] && return
  integer from=0
  [[ ${(L)goal[1,${#BUFFER}]} == ${(L)BUFFER} ]] && from=${#BUFFER}
  _wib_word_end "$goal" $from
  _wib_to_typing
  _wib_fill "${goal[1,REPLY]}"
  _wib_goal_buffer=$BUFFER
}

# ---------------------------------------------------------------- recording

_wib_preexec() {
  _wib_cmd=''
  # Respect the user's history filters: HIST_IGNORE_SPACE and HISTORY_IGNORE.
  [[ $1 == ' '* ]] && return
  [[ -n ${HISTORY_IGNORE-} && $1 == ${~HISTORY_IGNORE} ]] && return
  _wib_cmd=$1
  _wib_cmd_ts=$EPOCHSECONDS
}

_wib_precmd() {
  local st=$?
  _wib_fix=''
  [[ -n $_wib_cmd ]] || return 0
  # The command goes through stdin so a secret in it never shows in `ps`.
  print -rn -- "$_wib_cmd" | _wib_run record --stdin --notice-id $$ --ts $_wib_cmd_ts --exit $st \
    --cwd "$PWD" --period "${WIB_PERIOD:-30d}" &>/dev/null &!
  # "command not found": offer the command with its program name corrected.
  # Any other failure costs nothing.
  if (( st == 127 )); then
    _wib_fix=$(print -rn -- "$_wib_cmd" | _wib_run suggest-fix --stdin --cwd "$PWD" \
      --period "${WIB_PERIOD:-30d}" 2>/dev/null)
  fi
  _wib_cmd=''
  return 0
}

# ---------------------------------------------------------------- doctor

# What `wib doctor` cannot see from outside the shell, as key=value lines.
_wib_doctor_state() {
  local k
  print -r -- "zsh_version=$ZSH_VERSION"
  print -r -- "ZDOTDIR=${ZDOTDIR-}"
  print -r -- "HISTFILE=${HISTFILE-}"
  print -r -- "KEYTIMEOUT=${KEYTIMEOUT-}"
  for k in extended_history share_history inc_append_history; do
    print -r -- "$k=${options[$k]-}"
  done
  for k in '^[[B' '^[OB' '^[[Z' '^[' '^G' '^]'; do
    print -r -- "bind:$k=${${"$(bindkey -M main -- "$k")"}##* }"
  done
  for k in WIB_PERIOD WIB_LIMIT WIB_MIN_CHARS WIB_ZSH_HISTORY WIB_DATA_DIR WIB_SECRET_GUARD; do
    print -r -- "$k=${(P)k-}"
  done
}

# `wib doctor` also checks the shell it runs in; everything else is the binary
# this shell loaded (`command wib` runs whichever is on $PATH).
wib() {
  if [[ $1 == doctor ]]; then
    _WIB_DOCTOR_STATE="$(_wib_doctor_state)" _wib_run "$@"
  else
    "$_wib_bin" "$@"
  fi
}

# ---------------------------------------------------------------- wiring

_wib_bind_main() {
  local widget=$1 key
  shift
  for key in "$@"; do
    local current=${"$(bindkey -M main -- "$key")"##* }
    [[ $current == _wib_* ]] || _wib_orig[${(g:ce:)key}]=$current
    bindkey -M main -- "$key" "$widget"
  done
}

() {
  local w
  for w in _wib_down _wib_word _wib_dismiss _wib_slot _wib_nav_slot \
           _wib_nav_down _wib_nav_up _wib_nav_tab _wib_nav_enter _wib_nav_space _wib_nav_dismiss; do
    zle -N $w
  done

  _wib_bind_main _wib_down    '^[[B' '^[OB'
  _wib_bind_main _wib_word    '^[[Z'
  _wib_bind_main _wib_dismiss '^[' '^G'
  _wib_bind_main _wib_slot    '^]'

  _wib_make_nav_keymap

  add-zle-hook-widget line-pre-redraw _wib_pre_redraw
  add-zle-hook-widget line-init _wib_line_init
  add-zle-hook-widget line-finish _wib_line_finish
  add-zsh-hook preexec _wib_preexec
  add-zsh-hook zshexit _wib_zshexit
  # Run first so $? is still the exit status of the user's command.
  precmd_functions=( _wib_precmd ${precmd_functions:#_wib_precmd} )
}
