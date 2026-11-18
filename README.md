[English](README.md) | [简体中文](README.zh-CN.md)

<div align="center">
  <h1>BettRead</h1>
  <p>A browser extension that helps improve focus when reading web pages.</p>
  <p>BettRead highlights one sentence at a time and moves through the page with Vim-style keys.</p>
</div>

## Features

- `Alt+R` or the toolbar icon starts or stops reading mode.
- `j` moves to the next paragraph and selects its first sentence.
- `k` moves to the previous paragraph and selects its first sentence.
- `h` moves to the previous sentence; when on a paragraph's first sentence, it moves to the previous paragraph's last sentence.
- `l` moves to the next sentence; when on a paragraph's last sentence, it moves to the next paragraph's first sentence.
- Custom highlight style: underline or block.
- Custom highlight color: enter a hexadecimal value.

## Requirements

- Rust 1.88 or newer.
- The `wasm32-unknown-unknown` target.
- Oxichrome `0.2.1` from commit `f36ed22c007541e47308904ee925e21aa544e431`.
- A current Edge, Chrome, or Firefox release with CSS Custom Highlight support.

## Install the pinned Oxichrome CLI

The published `0.2.0` crate does not include the content-script macro (this can change after `0.2.1` is published on crates.io).

`cargo-oxichrome` must use the same revision as the library dependency:

```sh
cargo install --git https://github.com/0xsouravm/oxichrome --rev f36ed22c007541e47308904ee925e21aa544e431 cargo-oxichrome
cargo oxichrome --help
```

## Build

Install the WebAssembly target before the first build:

```sh
rustup target add wasm32-unknown-unknown
```

Build both browser products:

```sh
cargo oxichrome build
cargo oxichrome build --target firefox
```

> Outputs are written to `dist/chromium` and `dist/firefox`.

## Load the extension

### Edge or Chrome

1. Open `edge://extensions` or `chrome://extensions`.
2. Enable developer mode.
3. Choose **Load unpacked**.
4. Select `dist/chromium`.

> Chrome stable ignores `--load-extension` from the command line; use **Load unpacked**.

### Firefox

1. Open `about:debugging#/runtime/this-firefox`.
2. Choose **Load Temporary Add-on**.
3. Select `dist/firefox/manifest.json`.

> Firefox removes temporary add-ons when the browser exits.

## Development checks

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo check --workspace --all-targets
cargo doc --workspace --no-deps
```

## Project layout

- `src/`: Oxichrome extension entrypoints, DOM adapter, and reading-session lifecycle.
- `core/`: the `brcore` library with browser-independent reading models.
- `static/`: icons and options-page styles.
- `openspec/`: change specifications and task tracking.
- `dist/`: generated browser products, excluded from Git.
