# Voice command grammar, help, and push-to-talk

`crates/curbcut-tts/src/command.rs` is the deterministic layer on top of speech
recognition: a versioned phrase catalog, a "what can I say?" panel, and the
push-to-talk state machine. A voice command is matched by phrase, not by a
model, so the same grammar runs with any recognizer. `CC-084`.

Recognition upstream of this grammar is planned, not implemented; see
[`docs/VOICE_PIPELINE.md`](VOICE_PIPELINE.md).

## Rules

1. **The longest phrase wins.** `read faster` is `VoiceCommand::ReadFaster`,
   not `VoiceCommand::Read` with an argument, so a shorter phrase cannot
   swallow a longer one.
2. **Matching is on whole words.** A phrase matches only at a word boundary, so
   `read` does not match `reading` and `stop` does not match `stopwatch`.
3. **An unknown command is answered, not ignored.**
   `ParsedCommand::reply` returns a line that points at the help command.
4. **The utterance is not retained.** A `ParsedCommand` keeps the matched
   catalog phrase and the argument; `Debug` redacts the argument so a log line
   cannot print what was said.

## Normalization

`normalize` lowercases, trims, collapses whitespace, and treats `. , ! ? ; :`
as separators. `Read, please!` becomes `read please`, so it still matches the
`read` phrase and carries `please` as the argument.

## Catalog

`CATALOG` maps each command to its phrases. `VoiceCommand::ALL` fixes the order
the help panel lists, and `CommandGrammar::phrases_for` returns the phrases for
one command.

| Command | Phrases | Example |
| --- | --- | --- |
| `read` | `read aloud`, `read it`, `read this`, `start reading`, `read` | "read this" |
| `stop` | `stop reading`, `stop speaking`, `be quiet`, `stop`, `cancel` | "stop" |
| `pause` | `pause reading`, `pause`, `hold` | "pause" |
| `resume` | `keep reading`, `continue reading`, `resume`, `continue`, `go on` | "continue" |
| `simplify` | `make it simpler`, `simplify this`, `simplify`, `easier` | "simplify this" |
| `read-faster` | `read faster`, `speak faster`, `faster`, `speed up` | "faster" |
| `read-slower` | `read slower`, `speak slower`, `slower`, `slow down` | "slower" |
| `next` | `next item`, `skip forward`, `next`, `skip` | "next" |
| `previous` | `previous item`, `go back one`, `previous` | "previous" |
| `repeat` | `say that again`, `repeat that`, `repeat`, `again` | "repeat" |
| `back` | `go back`, `previous page`, `back` | "back" |
| `scroll-down` | `read down`, `scroll down` | "scroll down" |
| `scroll-up` | `read up`, `scroll up` | "scroll up" |
| `click` | `click it`, `click`, `tap`, `press` | "click" |
| `dismiss` | `close it`, `dismiss`, `close`, `hide` | "dismiss" |
| `show-answers` | `show my answers`, `show answers`, `your answers`, `answers` | "your answers" |
| `help` | `help`, `help me` | "help" |
| `what-can-i-say` | `what can i say`, `what are the commands`, `list commands`, `show commands` | "what can I say" |

## Matching

`CommandGrammar::parse` normalizes the utterance, then scans the catalog and
keeps the longest phrase that matches at a word boundary. The remainder after
the phrase is the argument. When nothing matches, the result is
`VoiceCommand::Unknown`.

`ParsedCommand` exposes `command`, `matched`, and `argument`. `reply` returns a
short confirmation for a known command and the unknown-command line otherwise.

## Help

`CommandGrammar::help` renders the full panel: one block per command with its
stable name, its description, and its phrases. `help_for` renders one line.
`UNKNOWN_REPLY` is the line for an unrecognized utterance:

> I didn't catch that. Say 'what can I say' to hear the commands.

## Push to talk

`PushToTalk` is a two-state machine with two modes.

| Mode | `press` | `release` |
| --- | --- | --- |
| `Hold` (default) | start listening | stop listening |
| `Toggle` | flip | no change |

`toggle` flips the state in either mode. `set_mode` switches the mode; switching
to `Hold` while listening stops listening. `presses` counts key presses and
`transitions` counts state changes, so a release while idle is not a transition.

## API

- `VoiceCommand::{ALL, as_str, description, examples, is_unknown}`.
- `CommandGrammar::{new, catalog, phrases_for, parse, help, help_for, unknown_reply}`.
- `ParsedCommand::{new, command, matched, argument, is_known, is_unknown, reply}`.
- `PushToTalk::{new, with_mode, mode, set_mode, state, is_listening, presses, transitions, press, release, toggle, reset}`.
- `PushToTalkMode`, `PushToTalkState`, `UNKNOWN_REPLY`.

## Privacy

The grammar stores the phrase catalog, which is public. A parsed command keeps
the matched phrase and the argument; the argument is user speech, not screen
content, and `Debug` redacts it. Nothing is written to disk. The workspace
canary scan covers this crate.
