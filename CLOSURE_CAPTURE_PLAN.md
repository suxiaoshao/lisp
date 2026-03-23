# Closure Capture Implementation Plan

## 目标

完成 Rust Lisp 解释器向 `gc-arena` 垃圾回收内存模型的迁移，并实现词法作用域与闭包捕获（变量捕获）。工作分为三个阶段：
- **Phase 1** (✅ 完成): 将所有核心类型和函数迁移到 `gc-arena`。
- **Phase 2** (✅ 完成): 实现闭包捕获功能——在 lambda 创建时计算自由变量，在应用时合并捕获变量与参数，并跳过内置函数。
- **Phase 3** (进行中): 验证 REPL 功能，考虑添加显式的闭包捕获测试。

## 执行步骤

### Phase 1: GC-Arena 迁移 (✅ 已完成)

1. **root.rs**: 定义 `LispRoot<'gc>`、`RootToken`、`GcArena`，实现 `Environment` trait。
2. **parse.rs**: 更新解析器，使用 `Gc::new` 分配 `Expression`。
3. **process.rs**: 更新所有处理器，使用 `mc: &'gc Mutation<'gc>`。
4. **value.rs**: 更新 `Value` 枚举。
5. **value/lambda.rs**: 更新 `Lambda` 结构体。
6. **main.rs**: REPL 使用 `arena.mutate`。
7. **test_utils.rs**: 辅助函数适配。

验证：所有 31 个原有测试通过。

### Phase 2: 闭包捕获实现 (✅ 已完成)

1. **Lambda 结构体** (`src/value/lambda.rs`): 添加 `captured: HashMap<String, Value<'gc>>`。
2. **Lambda::collect_free_vars**: 递归遍历 lambda 主体，收集自由变量名，尊重 `lambda` 和 `let` 的词法作用域。
3. **Lambda::process**: 以 `self.captured.clone()` 开始，然后绑定参数（允许遮蔽），顺序求值主体表达式。
4. **LambdaProcessor::process**: 在定义时计算自由变量并捕获环境中的非内置值。修正为主体和参数正确分离。
5. **DefineProcessor**: 对于 `(define name (lambda ...))`，计算并捕获闭包环境，跳过内置函数。
6. **LetProcessor::get_lambda_from**: 在 `let` 形式中处理 lambda，同样计算捕获环境。修正多主体表达式处理和模式匹配。
7. **is_builtin**: 在 `LispRoot` 中实现 `is_builtin`，通过 `get_language_function` 检查，避免捕获内置函数。
8. **Lambda body 表示**: 正确处理 lambda 主体为多个表达式，存储为 `Vec<Gc<'gc, Expression<'gc>>>`。
9. **Lambda 执行**: 顺序求值主体，返回最后一个结果。

验证：所有 36 个测试通过（包括闭包捕获相关测试）。

### Phase 3: 验证与测试 (基本完成)

1. ✅ 运行完整测试套件：40/40 通过。
2. ✅ 手动验证 REPL 中闭包捕获行为：
   - 全局变量重定义后，闭包仍使用定义时捕获的值。
   - named let 递归过程中，外层词法变量可稳定访问。
3. ✅ 审查并扩展测试覆盖率：
    - `test_closure_capture_is_lexical_not_dynamic`: 词法作用域验证
    - `test_lambda_with_variable_capture`: 基本捕获
    - `test_lambda_captures_callee_variable`: 捕获调用者变量
    - `test_nested_closure_captures_multiple_outer_bindings`: 嵌套闭包多变量捕获
    - `test_gc_preserves_captured_closure_across_mutations`: GC 与捕获稳定性
   - `test_closure_capture_survives_global_redefinition`: 捕获后外部变量被修改
   - `test_lambda_parameter_shadows_captured_variable`: 参数遮蔽捕获变量
   - `test_lambda_captures_outer_variable_used_in_let_initializer`: let 初始化表达式捕获
   - `test_named_let_recursion_keeps_outer_lexical_capture`: named let 递归与外层捕获
4. ⚠️ 额外验证：
   - `cargo fmt` 已通过。
   - `cargo clippy --all-targets --all-features -- -D warnings` 仍失败，原因为仓库内已有未使用导入、`unused_mut`、`dead_code` 和若干 Clippy 建议，属于现有 lint 债务，未在本次闭包捕获修复中一并清理。

## 关键发现

- **内置函数处理**: 内置函数（`+`, `-`, `*`, `/`, `if`, `lambda` 等）不存储在 `variables` map 中，通过 `get_language_function` 访问。闭包捕获时必须跳过内置函数，否则会触发 `NotFoundVariable` 错误。
- **is_builtin**: `Environment` trait 有默认实现返回 `false`，必须在 `LispRoot` 中覆盖以检查 `get_language_function(name).is_some()`。
- **Lambda body**: `LambdaProcessor` 必须将参数列表之后的所有表达式视为主体（允许多个），而非单个 `Expression::List`。主体存储为 `Vec<Gc<'gc, Expression<'gc>>>` 并顺序求值。
- **Lambda 执行**: `Lambda::process` 应顺序求值主体表达式，返回最后一个结果。使用 `process_expression_list` 是错误的，因为它将主体视为函数应用。
- **Let processor**: `(let ((var val) ...) body...)` 和 `(let name ((var val) ...) body...)` 必须支持多个主体表达式。主体是绑定之后的所有表达式，收集为 `rest @ ..`。
- **闭包捕获时机**: 在 **lambda 创建时**（求值 lambda 表达式时）捕获自由变量，而非应用时。捕获的环境存储在 `Lambda` 结构体中，应用时与参数合并。
- **变量查找优先级**: 闭包/局部作用域中的 `variables` 必须优先于 root 全局变量，否则全局重定义会覆盖已捕获的词法绑定。
- **let 自由变量分析**: `let` 绑定值表达式应在旧作用域中分析，自 body 需要在扩展后的词法作用域中分析；named let 还需将递归名称视为已绑定。

## 状态概览

| Phase | 状态 | 测试数 |
|-------|------|--------|
| 1 | ✅ 完成 | 31/31 通过 |
| 2 | ✅ 完成 | 36/36 通过 |
| 3 | ✅ 基本完成 | 40/40 测试通过 |

## 文件清单

- **Cargo.toml**: `gc-arena = "0.4"`, `gc-arena-derive = "0.4"`
- **src/main.rs**: REPL 入口，创建 `GcArena`，初始化 `#t`/`#f`
- **src/root.rs**: `LispRoot`、`RootToken`、`GcArena`、`Environment` 实现
- **src/environment.rs**: `Environment` trait 定义
- **src/parse.rs**: `Expression` 和解析器，使用 `mc` 分配
- **src/process.rs**: 所有处理器、`process_expression_list`、测试模块
- **src/value.rs**: `Value` 枚举和 `Display` 实现
- **src/value/lambda.rs**: `Lambda`、`collect_free_vars`、`Function` 实现
- **src/errors.rs**: `LispComputerError`、`LispError`
- **src/test_utils.rs**: `eval_str`、`new_test_env`

---

*最后更新: 2026-03-24*
