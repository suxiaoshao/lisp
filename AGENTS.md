# AGENTS.md

## 项目入口

Rust Lisp 解释器 workspace，包与依赖信息以 `Cargo.toml` 为准。REPL 入口在 `crates/lisp/src/main.rs`；解析、求值、值与闭包、GC root 与环境实现位于 `crates/lisp-core/src/`。

## 实现与语义约束

- 优先做最小、可验证的改动，保持模块、命名与错误建模约定；直接修复语义根因，避免临时 hack 或兜底掩盖错误。
- 新增依赖使用完整版本号；文档与配置使用 UTF-8、LF。
- 解析须考虑完整输入消费、转义、嵌套和错误输入；求值须考虑类型错误、作用域、闭包捕获及内置函数与用户函数的分派。
- `gc-arena` 对象须通过 root、闭包捕获或其他有效路径持有。修改 `lambda`、`let`、`define` 或环境查找时，确保词法作用域与自由变量捕获正确。

## 验证与 GitHub

- 行为变更复用或补齐相关语义回归覆盖，重点为解析、求值、闭包、GC 和错误分支；跨这些边界的变化按影响扩大到完整 `cargo test`。Rust 格式与严格 Clippy 按项目配置及适用 CI 检查，合入 `main` 以 `.github/workflows/ci.yml` 为准。
- 文档与指令改动仅检查相关结构、链接和差异；说明未完成的适用验证及原因。
- Issue、PR 和评论遵循 `.github/` 中适用模板，PR 模板为 `.github/pull_request_template.md`。PR 描述覆盖当前分支相对远程最新 `main` 的整体差异；提交只包含本次授权改动。
