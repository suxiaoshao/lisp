# AGENTS.md

本文件定义代码代理在本仓库中的默认工作规则，帮助代理快速理解项目边界、验证方式和 GitHub 流程。

## 1. 仓库概览

- 本仓库是一个用 Rust 实现的 Lisp 解释器。
- 当前是单 crate 项目，不是 workspace。
- 技术基线：
  - Rust Edition: `2024`
  - 关键依赖：`nom`、`gc-arena`、`thiserror`、`rustyline`
- 核心目录：
  - `src/main.rs`：REPL 入口
  - `src/parse.rs`：语法解析
  - `src/process.rs`：内置函数与求值流程
  - `src/value.rs`、`src/value/lambda.rs`：值系统与闭包
  - `src/root.rs`：`gc-arena` root 与环境实现

## 2. 代码修改原则

- 优先做最小、可验证的改动，避免无关重构。
- 保持现有模块边界、命名方式和错误建模。
- 不要通过兜底分支或临时 hack 掩盖语义错误；优先修根因。
- 涉及解释器行为变更时，必须补对应单元测试，尤其是解析、求值、闭包、GC 和错误分支。
- 不要修改与当前任务无关的文件格式、导入顺序或目录结构。
- 文档和配置文件统一使用 `UTF-8` 与 `LF`。

## 3. Rust 与解释器规则

- 新增依赖必须使用完整版本号。
- 解析器改动要同时考虑：
  - 完整输入是否被消费
  - 转义、嵌套结构和错误输入
- 求值器改动要同时考虑：
  - 类型错误
  - 作用域与闭包捕获
  - 内置函数与用户函数的分派
- `gc-arena` 相关代码要确保对象通过 root、闭包捕获或其他有效路径被持有，不要制造悬空引用式设计。
- 如果修改 `lambda`、`let`、`define` 或环境查找逻辑，至少补一个词法作用域或自由变量捕获测试。

## 4. GitHub 与协作规则

- 编写 issue、PR、评论前，先检查 `.github/` 下的模板。
- 运行 `gh` 相关命令时，默认申请沙盒外权限；不要假设 GitHub CLI 能在沙盒内正常访问认证状态、远程仓库或网络。
- issue 默认使用：
  - `.github/ISSUE_TEMPLATE/bug_report.yml`
  - `.github/ISSUE_TEMPLATE/feature_request.yml`
  - `.github/ISSUE_TEMPLATE/tech_request.yml`
- PR 默认使用 `.github/pull_request_template.md`。
- PR 描述必须基于“当前分支相对远程最新 `main`”的整体差异，不要只总结最后一次提交。
- 如果工作树里存在与当前任务无关的改动，不要擅自纳入提交。

## 5. 验证与 CI

- 行为或配置变更后，至少执行与改动直接相关的验证。
- 默认验证基线：
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- 如果某项验证未执行，必须说明原因。
- 合入 `main` 的改动默认应通过 `.github/workflows/ci.yml`。
