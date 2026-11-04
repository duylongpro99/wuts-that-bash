# Product Hunt launch kit

Copy and assets for launching wib on Product Hunt. Every claim here is backed by
`README.md` (benchmarks, key bindings, privacy notes). Update both together if behaviour changes.

Assets live in `marketing/product-hunt/`; their HTML sources are in `marketing/product-hunt/src/`.

| Asset | File | Product Hunt slot |
|---|---|---|
| Logo / thumbnail, 240×240 | `logo-240.png` | Thumbnail |
| Logo, 1024×1024 | `logo-1024.png` | Maker profile, social posts |
| Gallery I–VI, 2540×1520 (1270×760 at 2x) | `gallery-1@2x.png` … `gallery-6@2x.png` | Gallery, in order; image I is the first impression |
| Demo video, 1920×1080, about 34 s | `wib-demo.mp4` | Upload to YouTube (unlisted is fine), paste the link in the video slot |
| Demo GIF, 960×540, 12 fps, title card skipped | `wib-demo.gif` | README hero (GitHub can't play a repo MP4 inline); also usable as an animated gallery item |

## Name and tagline

- **Name:** wib — what is that bash?
- **Tagline (60 max):** `Your zsh history, surfaced the moment you start typing` (54)
- Alternatives:
  - `Type two letters, get the command you ran last Tuesday` (54)
  - `Shell history suggestions under your cursor, no Ctrl-R` (54)

## Description (260 max)

> wib shows your past zsh commands right under the prompt as you type. Type gi and see your
> recent git commands; type dcup for docker compose up. Ranked by frequency, recency and
> project. One tiny Rust binary, no daemon, 100% local. Free and MIT.

(244 characters, pasted as plain text.)

## Topics

Developer Tools, Open Source, Productivity, Terminal / Command Line, macOS.

## The problem it solves

You already know the command. You ran it yesterday. You just can't remember whether it was
`--force-with-lease` or `-f`, which port you mapped, or the exact path to that config file.
Today you press Ctrl-R and play guess-the-substring, scroll up through hundreds of lines with ↑,
or retype an 80-character `docker run` from memory. Every tool that helps makes you stop and
open a search screen first: you have to realise you need to search before you can search.

wib removes that step. The answer is on screen before you ask.

## Who it's for

- Developers, DevOps and SRE engineers, and data people who live in a zsh terminal on macOS or Linux.
- Anyone who reruns the same long `git`, `docker`, `kubectl`, `ssh` or `npm` commands with small changes.
- People who switch between several repos and want each project's commands to come first.
- Privacy-minded users who won't send their shell history to a cloud service.

Not for (yet): bash and fish users. wib needs zsh 5.3 or newer.

## Key features

- **Suggestions as you type.** Up to 8 matching commands appear under the prompt after two
  characters and refine with every keystroke. No key to open a search screen.
- **Smart matching.** Prefix matches first, then word-initial fuzzy: `gi st` → `git status`,
  `dcup` → `docker compose up`.
- **Ranked by you.** Frequent and recent commands rise to the top; commands you ran in the
  current git repo come first; duplicates collapse to one line; typos that never succeeded sink
  to the bottom (they are not hidden).
- **Keyword search.** `wib pull,master` finds commands containing both words, in any order.
- **Word-by-word fill.** Shift+Tab fills the picked command one word at a time, so an old
  command becomes a template you edit only at the end.
- **Doesn't break your habits.** ↑, Tab, Enter and Ctrl-R behave exactly as before until you
  press ↓. Works with oh-my-zsh, powerlevel10k, zsh-autosuggestions and vi mode.
- **Works on day one.** Your existing `~/.zsh_history` powers suggestions immediately.
- **Fast and light.** About 5–8 ms per lookup on ~10k commands, process startup included.
  One Rust binary with zero dependencies, no daemon, no database server.
- **Private by design.** No cloud, no account, no telemetry. The log is owner-only (0600) and
  respects `HISTORY_IGNORE` and leading-space commands. `wib forget` removes a suggestion.

## How it compares

| | How you invoke it | Suggestions | Multi-machine sync |
|---|---|---|---|
| **wib** | Appears as you type | Several lines, right under the prompt | No, local only |
| zsh-autosuggestions | Appears as you type | One grey hint | No |
| Ctrl-R / fzf | Press a key to open | Search list | No |
| atuin | Press a key to open | Search list | Yes |

## Maker's first comment (introduction)

Post this as the first comment the moment the launch goes live.

> Hey Product Hunt! 👋
>
> I built **wib** ("what is that bash?") because I kept losing the same fight with my terminal.
> I *know* I ran that `docker run` with the right ports last week. I just can't remember the
> flags. So I'd hit Ctrl-R, guess a substring, miss, guess again, and eventually retype it.
>
> wib flips that around: **you don't search, it just shows up.** Type two letters and your
> matching commands appear right under the prompt, ranked by how often and how recently you ran
> them, and by the project you're in. Press ↓ and Enter to run one, or Shift+Tab to fill it
> word by word and change only the end.
>
> A few things I cared about:
> - **Speed.** About 5–8 ms per keystroke lookup on ~10k commands. You never wait on it.
> - **Nothing running in the background.** One small Rust binary, no daemon, no database.
> - **Your history stays yours.** No cloud, no account, no telemetry. Ever.
> - **No new habits.** ↑, Tab, Enter and Ctrl-R work exactly as before.
>
> It works with zsh today (macOS and Linux, with oh-my-zsh, p10k and vi mode). Setup is one
> binary and one line in `~/.zshrc`, and your existing history works from minute one.
>
> It's free and open source (MIT). I'd love to hear what you think, especially which shell you'd
> want next and what your most-retyped command is. 🙏
>
> GitHub: https://github.com/duylongpro99/wuts-that-bash · Site: https://wib.pages.dev/

## Short posts for launch day

- **X / Bluesky:** I just launched wib on Product Hunt 🚀 Your zsh history, surfaced the moment
  you start typing. Type `gi`, see your recent git commands. One tiny Rust binary, no daemon,
  100% local, MIT. [PH link]
- **LinkedIn / dev communities:** Ctrl-R makes you realise you need to search before you can
  search. wib skips that: matching past commands appear under your zsh prompt as you type,
  ranked by frequency, recency and the project you're in. Free and open source. [PH link]

## Video script (what `wib-demo.mp4` shows)

1. Title: "You ran it last Tuesday. You just can't remember the flags. What was that bash?"
2. Type `gi`: four git commands appear. ↓ selects `git push origin master`, Enter runs it.
3. Type `dcup`: `docker compose up` is suggested with `d`, `c` and `up` underlined. ↓, Enter.
4. Type `wib pull,master`: commands containing both words. ↓, Enter runs `git pull origin master`.
5. Type `gi`, ↓, then Shift+Tab four times: `git` → `git push` → `git push origin` → `git push origin master`.
6. End card: logo, tagline, the four facts, and the GitHub URL.

The video is silent. Add a music bed in YouTube Studio or a video editor if you want sound.

## Regenerating the assets

The images and video are rendered from the HTML in `marketing/product-hunt/src/` with headless
Chrome (Playwright, `channel: "chrome"`): screenshot `logo.html` at 1024×1024,
`gallery.html?s=1` … `?s=6` at 1270×760 with device scale 2, and step `video.html` through
`window.renderAt(t)` at 30 fps, piping the frames to `ffmpeg -c:v libx264 -pix_fmt yuv420p`.
`logo-240.png` is `sips -z 240 240 logo-1024.png`. `wib-demo.gif` is
`ffmpeg -ss 4.8 -i wib-demo.mp4 -vf "fps=12,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=64:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle" wib-demo.gif`.

The README uses `logo-240.png`, `wib-demo.gif` and gallery images II–VI, so re-render them in place
rather than renaming.
