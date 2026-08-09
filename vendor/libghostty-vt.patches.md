# libghostty-vt local patches

This file tracks intentional local changes applied on top of the vendored
`libghostty-vt` source. Remove a patch only when the vendored source commit
contains the upstream behavior and the listed verification still passes.

## 0001 default lib-vt panes to grapheme clustering

status: active

patch: `vendor/patches/libghostty-vt/0001-default-grapheme-cluster-mode.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/243

upstream discussion: not opened; libghostty-vt currently exposes current mode mutation but no C API for configuring terminal default modes

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/terminal/c/terminal.zig`

reason: Herdr renders terminal cells directly and requires DEC private mode
2027 to store flags, ZWJ emoji, and other multi-codepoint grapheme clusters in
one cell. This patch makes clustering active for new terminals and keeps it as
the reset default so RIS (`ESC c`) does not disable it.

remove when: libghostty-vt exposes a C API for setting default mode 2027, or
upstream makes grapheme clustering the lib-vt default, and the reset-survival
regression passes without this patch.

verification:

```sh
cargo nextest run --locked grapheme_cluster_mode_is_default_and_survives_full_reset
cargo nextest run --locked grapheme_cluster_mode_renders_flag_emoji_in_single_wide_cell
cargo nextest run --locked grapheme_cluster_mode_renders_zwj_family_in_single_wide_cell
```

## 0002 preserve proxied Kitty key metadata

status: active

patch: `vendor/patches/libghostty-vt/0002-proxied-kitty-key-metadata.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/2514

upstream discussion: not opened; this extension is currently specific to terminal-proxy input

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/include/ghostty/vt/key/event.h`
- `vendor/libghostty-vt/src/input/key.zig`
- `vendor/libghostty-vt/src/input/key_encode.zig`
- `vendor/libghostty-vt/src/input/key_mods.zig`
- `vendor/libghostty-vt/src/lib_vt.zig`
- `vendor/libghostty-vt/src/terminal/c/key_event.zig`
- `vendor/libghostty-vt/src/terminal/c/main.zig`

reason: Herdr proxies rich Kitty key reports between terminals. The source event
can contain explicit shifted/base-layout alternates and Hyper/Meta modifiers
that libghostty-vt cannot reconstruct from local physical-key and layout data.
The extension preserves those fields so Ghostty can become Herdr's single pane
key encoder without losing protocol metadata.

remove when: upstream libghostty-vt exposes equivalent proxy-event alternate
codepoints and Hyper/Meta modifier support, and Herdr's encoder parity corpus
passes without this patch.

verification:

```sh
cd vendor/libghostty-vt && zig build test-lib-vt -Dsimd=true
just test-one keyboard_encoder_differences_are_explicit
just test-one keyboard_protocol_corpus
```
