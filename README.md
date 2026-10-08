# gray-session-name

Human names for sessions — `/name` plus a `prompt/context` hook. Port of pi's
`session-name` extension.

## Commands

- `/name <text>` — give the current session a friendly name
- `/name` — show the current name (or that none is set)
- `/name set` — interactive prompt via `host/ask`: the question "Name this
  session" is sent with an empty options list, so the free-form notes box
  is the input (`answers[qid].notes` wins; the first `answers` entry is the
  fallback). Without an ask channel it replies with the direct-set hint.

## Hooks

`prompt/context` returns `{text: "This session is named \"X\"."}` when the
session has a name. The host dedups injected context, so it surfaces once and
stays quiet after the first turn.

## State

`~/.gray/session-name/names.json` (honors `$GRAY_HOME`) — a
`{session_id: name}` map. Pi stored names in session metadata; the sidecar
keeps them keyed by `session.id` instead.

## Wire methods

`plugin/manifest`, `command/run`, `prompt/context`, `plugin/shutdown`.
Protocol 1.1. No tools. Capability `host.ask` powers `/name set` — without
it the command explains how to set the name directly.

## Install

```sh
gray plugin install session-name
gray plugin capabilities session-name --all   # grants host.ask for /name set
```

## Develop

```sh
cargo test
gray account check      # entry point + manifest handshake
gray account publish    # check → build → release → publish to the gray registry
```
