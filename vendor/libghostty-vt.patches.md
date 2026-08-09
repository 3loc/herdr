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
just test-one keyboard_corpus_survives_fragmentation_and_pane_encoding
```

## 0003 report Kitty repeat events

status: active

patch: `vendor/patches/libghostty-vt/0003-report-kitty-repeat-events.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/2514

upstream discussion: not opened

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/input/key_encode.zig`

reason: when Kitty event-type reporting is enabled, repeat events must remain
CSI-u events so applications can distinguish them from presses. Encoding a
repeat as plain text discards the event type at the pane boundary.

remove when: upstream libghostty-vt emits CSI-u for text-producing repeat
events whenever Kitty event-type reporting is enabled.

verification:

```sh
cd vendor/libghostty-vt && zig build test-lib-vt -Dsimd=true
just test-one keyboard_corpus_survives_fragmentation_and_pane_encoding
```

## 0004 encode extended function keys

status: active

patch: `vendor/patches/libghostty-vt/0004-encode-extended-function-keys.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/2514

upstream discussion: not opened

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/input/function_keys.zig`
- `vendor/libghostty-vt/src/input/key_encode.zig`

reason: libghostty-vt models F13-F25 but its legacy encoder has no entries for
them, silently suppressing keys that Herdr receives through Kitty input. The
extension uses the standard xterm/terminfo sequences, corrects modified F3 to
that same standard, and composes additional modifiers with each extended key's
implicit Shift or Control modifier. Modified F3 therefore shares the
`CSI 1;modifier R` byte shape used by a cursor position report, but terminal
input and terminal responses travel in opposite directions and are interpreted
in that context.

remove when: upstream libghostty-vt encodes F13-F25 in legacy mode with the
standard xterm sequences and modifier composition, and emits the standard
modified F3 sequence.

verification:

```sh
cd vendor/libghostty-vt && zig build test-lib-vt -Dsimd=true
just test-one keyboard_corpus_survives_fragmentation_and_pane_encoding
```

## 0005 preserve legacy Ctrl-Tab compatibility

status: active

patch: `vendor/patches/libghostty-vt/0005-preserve-legacy-ctrl-tab.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/2514

upstream discussion: not opened; this behavior preserves Herdr's existing
terminal-proxy compatibility policy rather than changing Ghostty's terminal UI
policy

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/input/function_keys.zig`
- `vendor/libghostty-vt/src/input/key_encode.zig`

reason: Herdr historically downgrades Ctrl-Tab to Tab for legacy destination
panes, which cannot distinguish Ctrl-Tab without an extended keyboard protocol.
Keeping this policy in the single Ghostty pane encoder avoids changing existing
shell and application behavior during the encoder cutover.

remove when: Herdr intentionally changes its legacy Ctrl-Tab compatibility
policy, with migration coverage for applications that currently receive Tab.

verification:

```sh
cd vendor/libghostty-vt && zig build test-lib-vt -Dsimd=true
just test-one ghostty_ctrl_tab_matches_the_pane_keyboard_protocol
just test-one keyboard_corpus_survives_fragmentation_and_pane_encoding
```

## 0006 honor consumed Shift in legacy control keys

status: active

patch: `vendor/patches/libghostty-vt/0006-honor-consumed-shift-in-legacy-control-keys.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/2514

upstream discussion: not opened; this changes proxy-event handling where Herdr
provides authoritative consumed-modifier metadata

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/input/key_encode.zig`

reason: for legacy destinations, a Shift modifier consumed to produce an
uppercase character must not turn Ctrl-Shift-C into CSI-u instead of the
traditional Ctrl-C byte. That legacy protocol cannot reliably preserve the
physical Shift after it produced uppercase text. modifyOtherKeys mode 2 still
receives every modifier, and Kitty panes still preserve Ctrl-Shift as distinct
metadata.

remove when: upstream libghostty-vt uses consumed modifier metadata for legacy
control-sequence selection while preserving modifyOtherKeys and Kitty behavior.

verification:

```sh
cd vendor/libghostty-vt && zig build test-lib-vt -Dsimd=true
just test-one retained_selection_copy_shortcut_requires_exact_modifiers
just test-one keyboard_corpus_survives_fragmentation_and_pane_encoding
```

## 0007 preserve proxy key compatibility

status: active

patch: `vendor/patches/libghostty-vt/0007-preserve-proxy-key-compatibility.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/2514

upstream discussion: not opened; these behaviors preserve Herdr's existing
terminal-proxy compatibility while the corresponding upstream encoder gaps are
reported separately

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/input/key_encode.zig`

reason: proxied semantic Alt events must retain their modifier with complete
UTF-8 text, and Windows enhanced input represents Ctrl-_ as semantic
Ctrl-minus. Without these extensions, legacy panes receive malformed UTF-8 or
CSI-u instead of unit separator.

remove when: upstream libghostty-vt prefixes complete UTF-8 text for semantic
Alt input and accepts Ctrl-minus as a unit-separator alias, with Herdr's
pipeline corpus passing unchanged.

verification:

```sh
cd vendor/libghostty-vt && zig build test-lib-vt -Dsimd=true
just test-one ghostty_legacy_pane_preserves_semantic_ctrl_minus_alias
just test-one keyboard_corpus_survives_fragmentation_and_pane_encoding
```
