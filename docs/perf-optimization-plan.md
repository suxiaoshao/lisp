# Lisp 性能优化路线图与第一阶段详细规格

## Summary
- 先做 `env` 重构，再做 `symbol interning`，最后按 profile 决定是否做 arena/布局压缩。
- 第一阶段的目标不是“局部微调”，而是把局部环境从“整表 clone”改成“链式 frame + 精确捕获”的模型，为第二阶段的 interned symbol 铺路。
- 语义保持不变：闭包仍是词法作用域、捕获仍是定义时快照、named `let` 的递归绑定仍只在函数体可见、builtin shadowing 行为不变。
- 第二阶段允许调整公开 parser API，直接把 interner 接入 parse 路径，不做兼容包装层。
- 第三阶段是 profile-gated：只有在 env + interning 完成后仍能证明分配/扫描是热点时才做。

## Performance Measurement and Benchmark Infrastructure

### Goals
- 性能测量基础设施必须同时服务三件事：本地比较、PR 门槛检查、以及详细 benchmark artifact 归档。
- 测量结果必须和语义正确性验证分离；`cargo fmt --check`、`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings` 仍然是功能正确性的唯一基线。
- 性能门槛默认以“同一 runner 上的 base vs head 对比”为准，不维护仓库内静态绝对 baseline。

### Command Surface
- 性能命令统一通过单个辅助 bin 暴露：
  - `cargo run --release --bin perf -- list`
  - `cargo run --release --bin perf -- capture --output <path> [--filter <pattern>]`
  - `cargo run --release --bin perf -- compare --base <path> --head <path> --thresholds perf/thresholds.tsv --markdown-out <path> --stage <env|symbol_interning|arena>`
  - `cargo run --release --bin perf -- bench [--filter <pattern>]`
  - `cargo run --release --bin perf -- ci --base <path> --head <path> --artifacts <dir> --stage <env|symbol_interning|arena>`
- `capture` 的 TSV 输出是 CI gate 的唯一输入。
- `compare` 负责 markdown 摘要和 fail/pass 判定。
- `bench` 只负责调用 Criterion 产出 `target/criterion`。
- `compare` 和 `ci` 都必须显式接收 `--stage`，避免三阶段门槛被同时套用。

### Scenario Matrix
- 场景 ID 必须稳定，后续只能追加，不能重命名已有项：
  - `parse_symbol_dense`
  - `parse_nested_forms`
  - `eval_arith_deep`
  - `eval_closure_chain`
  - `eval_named_let_loop`
  - `eval_do_loop`
  - `eval_shadow_builtin`
  - `eval_persistent_global_capture`
- `ParseOnly` 场景每次迭代新建 arena/root，只测 `parse_expression`。
- `EvalFresh` 场景每次迭代新建 arena/root，测 `parse + eval`。
- `EvalPersistent` 场景在单个 sample 内复用 arena/root，模拟 REPL 持久状态。
- 所有场景定义统一放在 `src/perf_support.rs`，禁止 `benches/` 和 `src/bin/perf.rs` 维护独立 workload。

### Local Workflow
1. 在基线分支运行 `cargo run --release --bin perf -- capture --output /tmp/base.tsv`。
2. 在目标分支运行 `cargo run --release --bin perf -- capture --output /tmp/head.tsv`。
3. 运行 `cargo run --release --bin perf -- compare --base /tmp/base.tsv --head /tmp/head.tsv --thresholds perf/thresholds.tsv --markdown-out /tmp/perf-summary.md --stage env`。
4. 运行 `cargo run --release --bin perf -- bench` 或直接 `cargo bench --bench perf_eval`。
5. 评审时同时查看 `/tmp/perf-summary.md` 和 `target/criterion`。

### CI Workflow
- 新增独立 `.github/workflows/perf.yml`，不并入主 CI。
- 只跑 Ubuntu。
- 触发方式：
  - `workflow_dispatch`
  - 命中核心解释器、benchmark 基础设施和门槛文件的 PR
- workflow 步骤：
  - checkout PR base 到独立目录
  - checkout PR head 到独立目录
  - 在 base/head 目录分别运行 `cargo run --release --bin perf -- capture`
  - 在 head 目录运行 `cargo run --release --bin perf -- compare --stage <active-stage>`
  - 在 head 目录运行 `cargo bench --bench perf_eval`
  - 上传 `base.tsv`、`head.tsv`、`perf-summary.md` 和 `target/criterion`
- 当前默认激活阶段是 `env`；推进到 Stage 2 或 Stage 3 时，同步更新 workflow 和文档中的 `--stage`。

### Gate Policy
- 门槛配置统一存放在 `perf/thresholds.tsv`。
- Env 阶段默认门槛：
  - `eval_closure_chain >= 20%`
  - `eval_named_let_loop >= 15%`
  - `eval_do_loop >= 10%`
  - `eval_arith_deep` 最大回退 `5%`
  - `eval_shadow_builtin` 最大回退 `5%`
- Symbol interning 阶段默认门槛：
  - `parse_symbol_dense >= 25%`
  - `parse_nested_forms >= 10%`
  - `eval_shadow_builtin >= 10%`
  - `eval_closure_chain` 最大回退 `5%`
- Arena 阶段默认门槛：
  - `eval_persistent_global_capture >= 8%`
  - `eval_closure_chain >= 8%`
  - `eval_do_loop` 最大回退 `5%`
- Stage 3 只有在 Stage 2 后的 Criterion 报表或 profile 仍显示 env frame 分配、hash 查找或 GC/扫描是热点时，才允许启用 Arena 阶段门槛。

### Bootstrap Policy
- 首个引入 `src/bin/perf.rs`、`criterion`、`perf/thresholds.tsv`、`perf.yml` 的 PR 不做硬门槛失败。
- 如果 base 分支缺少 `src/bin/perf.rs` 或 `perf/thresholds.tsv`，perf workflow 自动退化为 artifact-only 模式：只生成 head 的 capture 结果和 Criterion 报表，不执行 compare fail/pass。
- 从基础设施落地后的下一批优化 PR 开始启用硬门槛。

## Stage 1: Env 结构重构，去掉局部环境全量 clone
- 在 `root` 模块新增公开类型 `LocalEnv<'gc>`，作为局部环境句柄，替代所有 `&HashMap<String, Value<'gc>>` 参数。
- `LocalEnv<'gc>` 设计为一个轻量 handle，内部持有 `Option<Gc<'gc, EnvFrame<'gc>>>`。
- `EnvFrame<'gc>` 作为内部结构体，字段固定为：
  - `parent: Option<Gc<'gc, EnvFrame<'gc>>>`
  - `bindings: Gc<'gc, RefLock<HashMap<String, Value<'gc>>>>`
- `LocalEnv<'gc>` 暴露的方法固定为：
  - `pub fn empty() -> Self`
  - `pub fn extend_frame(&self, bindings: HashMap<String, Value<'gc>>, mc: &'gc Mutation<'gc>) -> Self`
  - `pub fn extend_one(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>) -> Self`
  - `pub fn lookup(&self, name: &str) -> Option<Value<'gc>>`
  - `pub fn insert_here(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>)`
- `insert_here` 只允许修改当前 frame，不沿父链写入；它只服务 `do` 循环的可变局部变量，不用于普通闭包或 `let` 捕获。

- `Expression::eval` 的签名改为：
  - `pub fn eval(&self, env: &'gc LispRoot<'gc>, locals: LocalEnv<'gc>, mc: &'gc Mutation<'gc>) -> Result<Value<'gc>, LispComputerError>`
- `ProcessorFunc` 的签名同步改为接收 `LocalEnv<'gc>`。
- `LispRoot::get_variable` 改为：
  - `pub fn get_variable(&self, name: &str, locals: LocalEnv<'gc>) -> Option<Value<'gc>>`
- `LispRoot::process_variable` 改为：
  - `pub fn process_variable(&'gc self, symbol: &str, args: &[Gc<'gc, Expression<'gc>>], locals: LocalEnv<'gc>, mc: &'gc Mutation<'gc>) -> Result<Value<'gc>, LispComputerError>`

- `Lambda<'gc>` 的结构改为：
  - `params: Vec<String>`
  - `body: Vec<Gc<'gc, Expression<'gc>>>`
  - `captured: LocalEnv<'gc>`
- `Lambda::new` 的第三个参数改为 `captured: LocalEnv<'gc>`。
- `Lambda::call` 改为双环境接口，明确区分“参数求值环境”和“函数体附加环境”：
  - `pub fn call(&self, args: &[Gc<'gc, Expression<'gc>>], env: &'gc LispRoot<'gc>, arg_env: LocalEnv<'gc>, body_tail_env: LocalEnv<'gc>, mc: &'gc Mutation<'gc>) -> Result<Value<'gc>, LispComputerError>`
- `Lambda::call` 的执行顺序固定为：
  1. 用 `arg_env` 求值所有实参。
  2. 校验 arity。
  3. 构造参数 frame。
  4. 函数体环境按 `params frame -> captured frame -> body_tail_env -> global env` 链接。
  5. 顺序求值 body，返回最后一个值。
- 普通 lambda 调用时传 `body_tail_env = LocalEnv::empty()`。
- named `let` 调用时只把递归名绑定放进 `body_tail_env`，不参与 initializer 求值。这样精确保留当前语义。

- 在 `LispRoot` 或 `process` 内新增一个集中式捕获辅助函数，名称固定为 `capture_env`：
  - 输入是自由变量名集合、当前 `LocalEnv`、`Mutation`
  - 输出是只包含这些自由变量快照的新 `LocalEnv`
  - 规则是按当前 lookup 结果复制 `Value`，不保存对原局部 frame 的可变别名
- `lambda_call`、`define_call`、`let_call` 的闭包捕获逻辑统一走 `capture_env`，不再手动构造 `HashMap<String, Value>`。

- `let_call` 的重构规则固定为：
  - simple `let`：先从 bindings 构造参数名列表与实参表达式列表，再构造一个 `Lambda`，最后用 `body_tail_env = LocalEnv::empty()` 调 `Lambda::call`
  - named `let`：先构造 `Lambda`，再创建一个只含递归名的 frame 作为 `body_tail_env`，并用外层 `locals` 作为 `arg_env`
- `do_call` 的重构规则固定为：
  - 以传入 `locals` 为 parent 创建一个新的可变 loop frame
  - init 表达式在外层 `locals` 中求值，结果写入 loop frame
  - test/body/step 均在 loop frame 对应的 `LocalEnv` 中执行
  - step 更新只写当前 loop frame，不改父环境

## Stage 2: Symbol Interning，全链路把 symbol 从 `String` 改成 interned handle
- 新增 `symbol` 模块，并引入两个类型：
  - `pub struct Symbol<'gc> { id: SymbolId, name: Gc<'gc, String> }`
  - `struct SymbolId(u32)`，保持 crate 内部使用，不对外暴露为主接口
- `Expression::Variable` 从 `Variable(String)` 改为 `Variable(Symbol<'gc>)`。
- `Lambda<'gc>` 的参数列表从 `Vec<String>` 改为 `Vec<Symbol<'gc>>`。
- 局部环境和全局环境的 key 统一改为 `SymbolId`。
- `LispRoot<'gc>` 新增字段：
  - `symbols: Gc<'gc, RefLock<SymbolTable<'gc>>>`
  - `variables: Gc<'gc, RefLock<HashMap<SymbolId, Value<'gc>>>>`
  - `builtins: BuiltinSymbols`
- `SymbolTable<'gc>` 的字段固定为：
  - `ids_by_name: HashMap<String, SymbolId>`
  - `names: Vec<Gc<'gc, String>>`
- `BuiltinSymbols` 固定缓存这些 interned id：`if`、`cond`、`lambda`、`define`、`let`、`do`、`and`、`or`、`+`、`-`、`*`、`/`、`=`、`>`、`<`、`>=`、`<=`、`#t`、`#f`。
- `LispRoot::new` 必须先初始化 symbol table，再 intern 所有 builtin，再填充全局环境。

- 公开 parser API 直接改为：
  - `pub fn parse_expression<'i, 'gc>(mc: &'gc Mutation<'gc>, root: &'gc LispRoot<'gc>, input: &'i str) -> IResult<&'i str, Gc<'gc, Expression<'gc>>>`
- `parse_lisp_variable` 仍解析原始文本，但在 `parse_expression` 中立即调用 `root.intern_symbol(name, mc)` 转成 `Symbol<'gc>`。
- `Display for Expression` 和 `Display for Lambda` 继续可读，直接使用 `Symbol.name` 输出，不依赖外部 root lookup。
- `LispRoot` 新增：
  - `pub fn intern_symbol(&self, name: &str, mc: &'gc Mutation<'gc>) -> Symbol<'gc>`
  - `fn get_variable_by_id(&self, symbol: SymbolId, locals: LocalEnv<'gc>) -> Option<Value<'gc>>`
- 错误类型不改结构，仍然保留 `String` 作为错误上下文；在构造错误时从 `Symbol.name` 转字符串。

- `Lambda::collect_free_vars` 改为基于 `SymbolId` 工作，并显式接收 `&BuiltinSymbols`：
  - `pub fn collect_free_vars(expr: &Expression<'gc>, builtins: &BuiltinSymbols, bound: &HashSet<SymbolId>, free: &mut HashSet<SymbolId>)`
- `process_expression_list`、`define_call`、`lambda_call`、`let_call`、`do_call` 全部改用 `SymbolId` 查找，不再在运行时做字符串 key 查找。

## Stage 3: Arena / 布局压缩，只在 profile 证明有价值时执行
- 触发条件固定为：完成 Stage 2 后，closure-heavy 和 symbol-heavy 场景仍有显著时间花在局部 frame 分配、hash map 分配或 GC/扫描；没有 profile 证据则直接跳过本阶段。
- 本阶段不再改语义和外部 API，只做内部布局压缩。
- `Lambda<'gc>` 的 `params` 和 `body` 改成不可变切片：
  - `params: Box<[Symbol<'gc>]>`
  - `body: Box<[Gc<'gc, Expression<'gc>>]>`
- `EnvFrame<'gc>` 改成 enum，区分 frozen frame 和 mutable frame：
  - `Frozen { parent, bindings: Box<[(SymbolId, Value<'gc>)]> }`
  - `Mutable { parent, bindings: Gc<'gc, RefLock<HashMap<SymbolId, Value<'gc>>>> }`
- 闭包捕获 frame 和参数 frame 一律使用 `Frozen`。
- `do` 循环的 frame 保持 `Mutable`。
- `LocalEnv::lookup` 调整为先匹配 frame 类型，再做线性查找或 hash 查找。
- 本阶段的成功标准是减少分配数和内存占用；如果只带来复杂度而无明显收益，则不合入。

## Test Plan
- 现有所有 parser、eval、closure、GC 测试必须保留并通过。
- Stage 1 必补测试：
  - `LocalEnv` lookup 优先级：当前 frame > captured frame > tail frame > global。
  - 普通 lambda 不依赖调用者局部变量，只依赖 captured + params。
  - named `let` 递归名只在 body 可见，不在 initializer 可见。
  - `do` 的 step 使用当前迭代值，且不会污染父环境。
  - 闭包在多次 arena mutation 后仍保活。
- Stage 2 必补测试：
  - 同一 root 下重复解析同名 symbol，得到同一个 interned `SymbolId`。
  - `Expression`/`Lambda` 的显示结果仍打印原始 symbol 文本。
  - 所有 `NotFoundVariable` / `UnboundFunction` / 参数错误仍输出原始文本名。
  - builtin shadowing、自由变量收集、named `let`、高阶函数场景在 interning 后行为不变。
- Stage 3 必补测试：
  - `Frozen` / `Mutable` frame lookup 语义一致。
  - `do` 的可变 frame 不会影响闭包快照 frame。
  - GC 保活和旧有闭包测试在新布局下全部通过。

## Performance Validation
- `src/bin/perf.rs` 必须覆盖这些单元测试：
  - TSV 解析与 round-trip
  - 门槛文件解析
  - base/head 比较逻辑
  - stage 过滤逻辑
  - markdown 摘要输出
- `cargo run --release --bin perf -- capture` 的输出必须至少包含：
  - `scenario_id`
  - `stage`
  - `kind`
  - `nanos_total`
  - `iterations`
  - `nanos_per_iter`
- `target/criterion` 是详细 benchmark artifact，不是门槛判定输入。
- perf workflow 的 pass/fail 只由 `compare` 决定，不直接解析 Criterion 报表。
- 现阶段的 compare/gate 默认只启用 `env` 阶段门槛。

## Assumptions
- 允许新增 `criterion` 作为唯一 benchmark 框架。
- 不引入第二套 benchmark 框架。
- 不引入 `xtask`；所有性能辅助命令统一通过单个 `perf` bin 暴露。
- 第一阶段允许调整公开 `Expression::eval` / `ProcessorFunc` 相关签名，并新增公开 `LocalEnv<'gc>`。
- 第二阶段允许破坏性修改公开 parser API。
- 错误类型保持字符串输出，不把 `SymbolId` 暴露到错误层。
- Stage 1 是必须落地的详细实施方案；Stage 2 是紧随其后的确定性方案；Stage 3 只有在 profile 证明有效时才实施。
- 默认验证基线保持为：`cargo fmt --check`、`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings`。
