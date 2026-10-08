<p align="center">
  <img src="assets/gray-logo.svg" alt="gray" width="96">
</p>
<h1 align="center">gray-session-name</h1>
<p align="center">Assign human-friendly names to gray sessions.</p>
<p align="center">
  <a href="https://github.com/vstaln/gray-session-name/blob/main/LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-7aa2f7.svg">
  <img alt="rust" src="https://img.shields.io/badge/built%20with-rust-orange.svg">
</p>

Give the current session a memorable name and surface it to the model when
the session starts.

## Commands

- `/name <text>` — set the current session's friendly name
- `/name` — show the current name, or report that none is set
- `/name set` — interactive prompt through `host/ask`: the free-form notes
  box supplies the name. Without an ask channel, the command explains how to
  set the name directly.

## Hooks

`prompt/context` returns `{text: "This session is named "X"."}` when a
name is set. The host deduplicates injected context, so it appears once and
stays quiet on later turns.

## State

`~/.gray/session-name/names.json` (honors `$GRAY_HOME`) stores a
`{session_id: name}` map.

## Wire methods

`plugin/manifest`, `command/run`, `prompt/context`, `plugin/shutdown`.
Protocol 1.1. No tools. Capability `host.ask` powers `/name set`; without it
the command explains how to set the name directly.

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

---
Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
