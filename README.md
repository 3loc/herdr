# herdr

> [!IMPORTANT]
> This is the public [3LOC fork](https://github.com/3loc/herdr) of
> [Herdr](https://github.com/herdrdev/herdr). It follows upstream while adding
> the features described below.

## what this fork adds

- **keyboard navigation for the whole sidebar**: press `ctrl+b`, then `h`, and
  use `j`/`k` to move across spaces and agents. Press `l` or Enter to activate.
- **pane notes for busy agents**: press `ctrl+b a` to queue a thought without
  interrupting the current turn. Notes stay visible in a bordered pane card,
  can be opened with `ctrl+b m`, and are delivered as the agent becomes ready.
- **a full-screen keybinding reference**: press `ctrl+b ?` for a searchable,
  multi-column view of the active keymap.

The underlying runtime, protocol, documentation, and most features come from
upstream Herdr. Fork-specific changes live on the default branch.

<p align="center">
  <img src="assets/logo.png" alt="herdr" width="100" />
</p>

<p align="center">
  <a href="https://herdr.dev">herdr.dev</a> · <a href="#install">install</a> · <a href="https://herdr.dev/docs/quick-start/">quick start</a> · <a href="https://herdr.dev/docs/">docs</a>
</p>

<p align="center">
  English · <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-666666?labelColor=333333" alt="Apache 2.0 license" /></a>
  <a href="https://github.com/3loc/herdr/releases"><img src="https://img.shields.io/github/downloads/3loc/herdr/total?labelColor=333333&color=666666" alt="3LOC fork downloads" /></a>
  <a href="https://github.com/3loc/herdr/stargazers"><img src="https://img.shields.io/github/stars/3loc/herdr?labelColor=333333&color=666666&logo=github" alt="3LOC fork GitHub stars" /></a>
  <a href="https://github.com/3loc/herdr/releases/latest"><img src="https://img.shields.io/github/v/release/3loc/herdr?label=3loc%20release&labelColor=333333&color=666666" alt="latest 3LOC fork release" /></a>
  <a href="https://x.com/herdrdev"><img src="https://img.shields.io/badge/follow-%40herdrdev-000000?logo=x&logoColor=white" alt="follow @herdrdev on X" /></a>
</p>

---

https://github.com/user-attachments/assets/043ec09f-4bdd-41d5-aee0-8fda6b83e267

**the runtime your coding agents live on.**

- **always running**: herdr is a background server; the terminals live inside it. close the lid, drop the network, or restart the machine; agents keep working and sessions come back. reattach from any terminal, or over ssh.
- **never hunt for the stuck one**: every pane is marked working, blocked, or idle. when an agent stops and needs an answer, herdr says so.
- **agent-native**: agents drive herdr through the cli and socket api: they can spawn panes, prompt each other, and wait until another agent is genuinely blocked. [agent skill →](https://herdr.dev/docs/agent-skill/)
- **runs what you already run**: claude code, codex, cursor, opencode, grok and the rest. herdr doesn't wrap or replace them; it owns their terminals.
- **keyboard and mouse, both first-class**: tmux-style prefix keys *and* click, drag, split. pick per moment, not per tool.
- **plugins**: extend panes and workflows. [browse the marketplace →](https://herdr.dev/plugins/)
- **one rust binary, no electron**: runs in whatever terminal you already use.

---

## install

Install the 3LOC fork on Linux or macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/3loc/herdr/master/scripts/install-3loc.sh | sh
```

The installer selects the correct x86-64 or ARM64 binary, verifies its SHA-256
checksum, and puts `herdr` in `~/.local/bin`. Set `HERDR_INSTALL_DIR` to choose
another directory. Fork builds update only from 3LOC releases.

Windows users can download `herdr-windows-x86_64.zip` from the
[latest 3LOC release](https://github.com/3loc/herdr/releases/latest). All fork
binaries are available on the [releases page](https://github.com/3loc/herdr/releases).

To install upstream Herdr instead, use the official
[installation instructions](https://herdr.dev/docs/quick-start/).

then start it where the work lives:

```bash
herdr
```

run your agents, split panes, walk away. `ctrl+b q` detaches, `herdr` reattaches. [quick start →](https://herdr.dev/docs/quick-start/)

## docs

everything lives at [herdr.dev/docs](https://herdr.dev/docs/): [quick start](https://herdr.dev/docs/quick-start/) · [concepts](https://herdr.dev/docs/concepts/) · [supported agents](https://herdr.dev/docs/agents/) · [keyboard](https://herdr.dev/docs/keyboard/) · [configuration](https://herdr.dev/docs/configuration/) · [session state](https://herdr.dev/docs/session-state/) · [remote](https://herdr.dev/docs/persistence-remote/) · [integrations](https://herdr.dev/docs/integrations/) · [plugins](https://herdr.dev/docs/plugins/) · [socket api](https://herdr.dev/docs/socket-api/)

## thanks

every past sponsor and backer is listed in [SPONSORS.md](./SPONSORS.md). thank you 🐑

enterprise / partnership: hey@herdr.dev

## agent instructions

if you are an ai agent helping with this repository, read [`AGENTS.md`](./AGENTS.md) before making changes and read [`CONTRIBUTING.md`](./CONTRIBUTING.md) before opening issues or PRs.

## development

```bash
git clone https://github.com/3loc/herdr
cd herdr
cargo build --release

just test        # unit tests
just check       # formatting, tests, and maintenance checks
```

## license

Herdr is licensed under the [Apache License 2.0](LICENSE).
