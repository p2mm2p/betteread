[English](README.md) | [简体中文](README.zh-CN.md)

<div align="center">
  <h1>BettRead</h1>
  <p>一款帮助提高网页阅读专注度的浏览器插件。</p>
  <p>BettRead 逐句高亮网页文本，并使用 Vim 风格按键移动阅读位置。</p>
</div>

## 功能

- 快捷键 `Alt+R` 或点击工具栏图标，开启或停止阅读模式。
- 使用 `j` 移动到下一段，并置于首句。
- 使用 `k` 移动到上一段，并置于首句。
- 使用 `h` 移动到上一句，处于段首句时移动至上一段末句。
- 使用 `l` 移动到下一句，处于段末句时移动至下一段首句。
- 自定义高亮样式，支持下划线和块。
- 自定义高亮颜色，支持输入十六进制色值。

## 环境要求

- Rust 1.88 或更高版本。
- `wasm32-unknown-unknown` target。
- Oxichrome `0.2.1`，固定 commit `f36ed22c007541e47308904ee925e21aa544e431`。
- 支持 CSS Custom Highlight 的新版 Edge、Chrome 或 Firefox。

## 安装固定版本的 Oxichrome CLI

已发布的 `0.2.0` crate 不包含内容脚本宏（后续 crate.io `0.2.1` 后可更换）。

`cargo-oxichrome` 必须与库依赖使用同一 revision：

```sh
cargo install --git https://github.com/0xsouravm/oxichrome --rev f36ed22c007541e47308904ee925e21aa544e431 cargo-oxichrome
cargo oxichrome --help
```

## 构建

首次构建前安装 WebAssembly target：

```sh
rustup target add wasm32-unknown-unknown
```

构建两个浏览器产物：

```sh
cargo oxichrome build
cargo oxichrome build --target firefox
```

> 产物位于 `dist/chromium` 和 `dist/firefox`。

## 加载扩展

### Edge 或 Chrome

1. 打开 `edge://extensions` 或 `chrome://extensions`。
2. 开启开发者模式。
3. 点击“加载已解压的扩展程序”。
4. 选择 `dist/chromium`。

> Chrome 稳定版会忽略命令行的 `--load-extension`，请使用“加载已解压的扩展程序”。

### Firefox

1. 打开 `about:debugging#/runtime/this-firefox`。
2. 点击“临时载入附加组件”。
3. 选择 `dist/firefox/manifest.json`。

> Firefox 退出后会移除临时附加组件。

## 开发检查

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo check --workspace --all-targets
cargo doc --workspace --no-deps
```

## 项目结构

- `src/`：Oxichrome 扩展入口、DOM 适配和阅读会话生命周期。
- `core/`：`brcore` 库，包含与浏览器无关的阅读模型。
- `static/`：图标和设置页样式。
- `openspec/`：变更规格和任务记录。
- `dist/`：生成的浏览器产物，不纳入 Git。
