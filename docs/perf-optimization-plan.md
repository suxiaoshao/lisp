# Lisp 性能优化路线图与阶段详细规格

## Summary
- 先做 `env` 重构，再做 `symbol interning + slot resolution`，最后按 profile 决定是否做 arena/布局压缩。
- 第一阶段的目标不是“局部微调”，而是把局部环境从“整表 clone”改成“链式 frame + 精确捕获”的模型，为第二阶段的 interned symbol 铺路。
- 语义保持不变：闭包仍是词法作用域、捕获仍是定义时快照、named `let` 的递归绑定仍只在函数体可见、builtin shadowing 行为不变。
- 第二阶段允许调整公开 parser API，直接把 interner 和 resolver 接入 parse 路径，不做兼容包装层。
- 第三阶段是 profile-gated：只有在 env + interning 完成后仍能证明分配/扫描是热点时才做。
- `Stage 1` 已完成，但 benchmark 表明它是“闭包/捕获路径有收益、循环/递归路径有回退”的中间态，不是最终性能形态。

## Performance Measurement and Benchmark Infrastructure

### Goals
- 性能测量基础设施必须同时服务三件事：本地比较、PR 门槛检查、以及详细 benchmark artifact 归档。
- 测量结果必须和语义正确性验证分离；`cargo fmt --check`、`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings` 仍然是功能正确性的唯一基线。
- 性能门槛默认以“同一 runner 上的 base vs head 对比”为准，不维护仓库内静态绝对 baseline。

### Command Surface
- 性能命令统一通过单个辅助 bin 暴露：
  - `cargo run --release --bin perf -- list`
  - `cargo run --release --bin perf -- capture --output <path> --stage <env|symbol_interning|arena> [--filter <pattern>]`
  - `cargo run --release --bin perf -- compare --base <path> --head <path> --thresholds perf/thresholds.tsv --markdown-out <path> --stage <env|symbol_interning|arena>`
  - `cargo run --release --bin perf -- bench --stage <env|symbol_interning|arena> [--filter <pattern>]`
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
  - `env_closure_deep_capture_{16|64|128}`
  - `env_closure_fanout_many_functions_{16|64|128}`
  - `env_lookup_chain_deep_{16|64|128}`
  - `env_named_let_wide_{16|64|128}`
  - `env_do_wide_{16|64|128}`
  - `env_capture_sparse_use_{16|64|128}`
  - `slot_closure_deep_capture_{16|64|128}`
  - `slot_lookup_chain_{16|64|128}`
  - `slot_named_let_wide_{16|64|128}`
  - `slot_do_wide_{16|64|128}`
- `ParseOnly` 场景每次迭代新建 arena/root，只测 `parse_expression`。
- `EvalFresh` 场景每次迭代新建 arena/root，测 `parse + eval`。
- `EvalPersistent` 场景在单个 sample 内复用 arena/root，模拟 REPL 持久状态。
- 所有场景定义统一放在 `src/perf_support.rs`，禁止 `benches/` 和 `src/bin/perf.rs` 维护独立 workload。
- `env_*` 场景用于放大局部环境重构的收益与回退：
  - `closure_deep_capture` 和 `closure_fanout_many_functions` 主要观察闭包创建/调用时的环境复制成本
  - `lookup_chain_deep` 主要观察链式环境查找的额外成本
  - `named_let_wide` 与 `do_wide` 主要观察循环场景下的 frame 查找与更新成本
  - `capture_sparse_use` 主要观察“大环境但只使用少量自由变量”时的收益
- `slot_*` 场景用于验证 `symbol interning + slot resolution` 是否真的替换了运行时字符串查找：
  - `slot_closure_deep_capture` 主要观察 local slot 读取是否保住 Stage 1 在闭包路径上的收益
  - `slot_lookup_chain` 主要观察 `depth + slot` 是否真正替代链式字符串查找
  - `slot_named_let_wide` 与 `slot_do_wide` 主要观察 Stage 1 当前回退路径是否被本地 slot 查找修复

### Local Workflow
1. 在基线分支运行 `cargo run --release --bin perf -- capture --output /tmp/base.tsv --stage env`。
2. 在目标分支运行 `cargo run --release --bin perf -- capture --output /tmp/head.tsv --stage env`。
3. 运行 `cargo run --release --bin perf -- compare --base /tmp/base.tsv --head /tmp/head.tsv --thresholds perf/thresholds.tsv --markdown-out /tmp/perf-summary.md --stage env`。
4. 运行 `cargo run --release --bin perf -- bench --stage env` 或直接 `cargo bench --bench perf_eval`。
5. 评审时同时查看 `/tmp/perf-summary.md` 和 `target/criterion`。
- Stage 2 本地比较时把 `env` 改成 `symbol_interning`，并优先单独跑 `parse_*` 与 `slot_*` 过滤器。

### CI Workflow
- 新增独立 `.github/workflows/perf.yml`，不并入主 CI。
- 只跑 Ubuntu。
- 触发方式：
  - `workflow_dispatch`
  - 命中核心解释器、benchmark 基础设施和门槛文件的 PR
- workflow 步骤：
  - checkout PR base 到独立目录
  - checkout PR head 到独立目录
  - 在 base/head 目录分别运行 `cargo run --release --bin perf -- capture --stage <active-stage>`
  - 在 head 目录运行 `cargo run --release --bin perf -- compare --stage <active-stage>`
  - 在 head 目录运行 `cargo run --release --bin perf -- bench --stage <active-stage>`
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
  - `slot_lookup_chain_64 >= 20%`
  - `slot_named_let_wide_64 >= 15%`
  - `slot_do_wide_64 >= 10%`
  - `slot_closure_deep_capture_64` 最大回退 `5%`
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

## Stage 1 Findings and Next-Step Design Guidance

### Measured Env Tradeoffs
- 最近一次本地 benchmark 对比表明，`Stage 1` 的收益主要集中在 `closure-heavy` 和 `capture-heavy` 场景，不是全路径提速。
- `env_closure_deep_capture`、`env_closure_fanout_many_functions`、`env_capture_sparse_use` 在 `16/64/128` 三档上都有稳定收益，说明“避免整表 clone”在闭包创建和调用路径上是有效的。
- `env_lookup_chain_deep` 只有小幅收益或接近平稳，说明链式 `LocalEnv` 本身不是长期性能终态。
- `env_named_let_wide` 和 `env_do_wide` 在宽环境、长循环场景上有明显回退，说明当前实现把一部分 clone 成本换成了更昂贵的 frame 查找和更新成本。
- 这些结论只作为后续设计依据，不直接变成新的 CI gate 规则；具体百分比以最近一次本地对比和 `perf` artifact 为准。

### How Other GC Languages Usually Solve This
- 链式环境 frame 通常只是基础结构，用来保证词法作用域和便宜地创建新作用域，但很少作为长期终态。
- 成熟实现通常会让闭包只捕获自由变量，而不是捕获整张环境；这比单纯微调 env 链更直接地降低 closure 大小、GC scan 和 lookup 范围。
- 对会被闭包共享或更新的绑定，常见做法是引入 cell / upvalue，而不是继续把所有局部变量都放在统一的 map/frame 里处理。
- 运行时变量查找通常会从 `String` key 进一步推进到 interned symbol、lexical address 或 slot index，避免热路径上的字符串哈希和多层 map 查找。
- 闭包很多的实现通常会走 flat closure / closure conversion，把自由变量压缩到紧凑的 closure env 里，而不是长期依赖宽 env 链。
- 普通局部变量、被捕获局部变量、全局变量和递归绑定通常会逐步分流到不同表示；“链式 `HashMap<String, Value>` env”更像过渡数据结构。

### Implications for Stage 2 and Stage 3
- `Stage 2` 不应只停留在 `symbol interning`；它还要为“从字符串查找到 symbol/slot 查找”铺路。
- `Stage 2` 要直接落地 `symbol interning + lexical address / slot`，而不是只替换 AST 里的 `String`。
- `Stage 2` 的设计默认要支持“精确自由变量捕获”，不要假设 closure 会长期依赖宽 `LocalEnv` 链。
- `Stage 3` 的前置条件不变，但 profile 时必须单独检查 `named let` / `do` 的 lookup 和 frame update 热点，不能只看 closure-heavy 场景。
- 如果后续还要继续优化 env，优先方向是“精确捕获 + symbol/slot + 语义分流”，而不是继续长期微调 `HashMap` 链本身。

## Stage 2: Symbol Interning + Slot Resolution

### Stage Goal
- `Stage 2` 的目标不是单纯减少 parse 时的 `String` 分配，而是同时消掉三类热路径：
  - parse 阶段重复 symbol 分配和比较
  - eval 热路径上的 `String` / `HashMap<String, _>` 查找
  - `named let`、`do`、嵌套 lambda 中的局部变量链式按名查找
- `Stage 2` 完成后，局部变量访问默认应当是“按 `depth + slot` 定位”，而不是“按名字沿 frame 链查找”。
- 语义保持不变：闭包仍是定义时快照，`named let` 递归名只在 body 可见，builtin shadowing 行为不变。

### Core Types and Runtime Structures
- 新增 `src/symbol.rs`，引入这些类型：
  - `pub struct SymbolId(u32)`
  - `pub struct Symbol<'gc> { id: SymbolId, name: Gc<'gc, String> }`
  - `pub struct LocalSlot<'gc> { symbol: Symbol<'gc>, depth: u16, slot: u16 }`
  - `pub enum ResolvedVar<'gc> { Global(Symbol<'gc>), Local(LocalSlot<'gc>) }`
  - `struct SymbolTable<'gc> { ids_by_name: HashMap<String, SymbolId>, names: Vec<Gc<'gc, String>> }`
  - `struct BuiltinSymbols { if_, cond, lambda, define, let_, do_, and_, or_, add, sub, mul, div, eq, gt, lt, ge, le, true_, false_ }`
- `Expression<'gc>` 固定改成：
  - `Number(f64)`
  - `String(Gc<'gc, String>)`
  - `Symbol(Symbol<'gc>)`
  - `Variable(ResolvedVar<'gc>)`
  - `List(Vec<Gc<'gc, Expression<'gc>>>)`
- `Symbol` 只负责“名字 + interned id”的稳定表示；进入可求值位置后统一降为 `ResolvedVar`。
- `EnvFrame<'gc>` 在本阶段固定改成 slot-backed 结构：
  - `parent: Option<Gc<'gc, EnvFrame<'gc>>>`
  - `layout: Gc<'gc, FrameLayout<'gc>>`
  - `values: Gc<'gc, RefLock<Vec<Value<'gc>>>>`
- `FrameLayout<'gc>` 固定包含：
  - `symbols: Box<[Symbol<'gc>]>`
  - `slot_by_id: HashMap<SymbolId, u16>`
  - `mutable: bool`
- `LocalEnv<'gc>` 保留 handle 语义，但公开方法调整为：
  - `pub fn empty() -> Self`
  - `pub fn extend_slots(&self, layout: Gc<'gc, FrameLayout<'gc>>, values: Vec<Value<'gc>>, mc: &'gc Mutation<'gc>) -> Self`
  - `pub fn lookup_local(&self, local: LocalSlot<'gc>) -> Option<Value<'gc>>`
  - `pub fn lookup_symbol(&self, symbol: SymbolId) -> Option<Value<'gc>>`
  - `pub fn set_slot_here(&self, slot: u16, value: Value<'gc>, mc: &'gc Mutation<'gc>)`
- `Frozen/Mutable` frame 二分仍留给 `Stage 3`；`Stage 2` 只把按名查找替换成 slot-backed frame。

### Parser and Resolver Pipeline
- 公开 parser API 直接改为：
  - `pub fn parse_expression<'i, 'gc>(mc: &'gc Mutation<'gc>, root: &'gc LispRoot<'gc>, input: &'i str) -> IResult<&'i str, Gc<'gc, Expression<'gc>>>`
- 解析流程固定为两段：
  - parse 阶段：所有标识符先解析成 `Expression::Symbol(Symbol<'gc>)`
  - resolve 阶段：遍历 AST，把可求值位置的 symbol 重写成 `Expression::Variable(ResolvedVar<'gc>)`
- 新增内部 resolver：
  - `struct Resolver<'gc> { root: &'gc LispRoot<'gc>, scopes: Vec<ResolverFrame<'gc>> }`
  - `struct ResolverFrame<'gc> { bindings: Vec<Symbol<'gc>>, slot_by_id: HashMap<SymbolId, u16> }`
- resolver 规则固定为：
  - 参数列表、`let` 绑定名、`define` 名称、`do` 变量名保留为 `Expression::Symbol`
  - 普通求值位置统一重写为 `Expression::Variable`
  - 命中 lexical scope 时重写为 `ResolvedVar::Local(LocalSlot)`
  - 否则重写为 `ResolvedVar::Global(Symbol)`
- 特殊形式识别规则固定为：
  - 只有当 list head 仍然指向当前 root 中的 builtin special form symbol 时，resolver 才按 `lambda`、`define`、`let`、`do`、`if`、`cond`、`and`、`or` 的作用域规则处理
  - 若该名字已经被局部或全局用户值 shadow，则按普通函数调用解析
- `named let` 的 resolve 规则：
  - 递归名只加入 body scope，不加入 initializer scope
  - 绑定项 value 在外层 scope resolve
  - body 和递归调用位置 resolve 到同一个 local slot
- `do` 的 resolve 规则：
  - `init` 在外层 scope resolve
  - `test`、`result`、`body`、`step` 都在 loop frame scope resolve
  - 每个 loop 变量固定拥有当前 frame 的一个 slot

### Runtime Lookup and Closure Capture
- `LispRoot<'gc>` 新增字段：
  - `symbols: Gc<'gc, RefLock<SymbolTable<'gc>>>`
  - `variables: Gc<'gc, RefLock<HashMap<SymbolId, Value<'gc>>>>`
  - `builtins: BuiltinSymbols`
- `LispRoot::new` 必须先初始化 symbol table，再 intern 所有 builtin symbol，最后填充全局环境。
- `LispRoot` 新增或替换这些方法：
  - `pub fn intern_symbol(&self, name: &str, mc: &'gc Mutation<'gc>) -> Symbol<'gc>`
  - `pub fn get_global(&self, id: SymbolId) -> Option<Value<'gc>>`
  - `pub fn set_global(&self, symbol: Symbol<'gc>, value: Value<'gc>, mc: &'gc Mutation<'gc>)`
  - `pub fn process_symbol(&'gc self, symbol: Symbol<'gc>, args: &[Gc<'gc, Expression<'gc>>], locals: &LocalEnv<'gc>, mc: &'gc Mutation<'gc>) -> Result<Value<'gc>, LispComputerError>`
- `Expression::eval` 的运行时分派固定为：
  - `Expression::Variable(ResolvedVar::Local(local))` 走 `locals.lookup_local(local)`
  - `Expression::Variable(ResolvedVar::Global(symbol))` 走 `env.get_global(symbol.id)`
  - `Expression::Symbol(_)` 在 eval 阶段视为内部错误路径，不作为正常可求值输入
- `Value::Processor` 改为携带 `Symbol<'gc>`，不再只携带 `&'static str`。
- `Lambda<'gc>` 固定改成：
  - `params: Box<[Symbol<'gc>]>`
  - `body: Box<[Gc<'gc, Expression<'gc>>]>`
  - `captured: LocalEnv<'gc>`
- `Lambda::collect_free_vars` 不再是 Stage 2 的主算法；resolver 直接产出 `CaptureSpec<'gc>`。
- `CaptureSpec<'gc>` 固定定义为：
  - `symbol: Symbol<'gc>`
  - `source: CaptureSource<'gc>`
- `CaptureSource<'gc>` 固定定义为：
  - `Local(LocalSlot<'gc>)`
  - `Global(Symbol<'gc>)`
- `capture_env` 固定改成“按 `CaptureSpec` 拷贝值”，不再接受 `HashSet<String>`。
- 关键语义保持不变：
  - 闭包仍然是定义时快照
  - 当前解释器对全局引用也保持定义时快照，不能在 Stage 2 被偷偷改成动态全局查找

### Change Scope
- `src/symbol.rs`
  - 新增 interner、`SymbolId`、`Symbol`、builtin symbol registry。
- `src/parse.rs`
  - parser 输出 raw symbol AST，并接 resolver 产出带 `ResolvedVar` 的最终 AST。
- `src/root.rs`
  - 全局变量从 `HashMap<String, Value>` 改成 `HashMap<SymbolId, Value>`，本地环境改成 slot-backed frame，新增 interner 和 builtin registry。
- `src/value.rs`、`src/value/lambda.rs`
  - `Value::Processor` 持有 `Symbol`，`Lambda` 参数和显示改成基于 `Symbol`，capture 改为 `CaptureSpec` 驱动。
- `src/process.rs`
  - 所有绑定位置从 `Expression::Variable(String)` 提取改成 `Expression::Symbol(Symbol)`；所有可求值变量分派改为 `ResolvedVar`；移除以 `HashSet<String>` 为中心的 free-var 分析。
- `src/lib.rs`、`src/main.rs`、`src/test_utils.rs`
  - 对外 parser API、示例代码和测试辅助同步适配 `parse_expression(mc, root, input)`。
- `src/perf_support.rs`、`src/bin/perf.rs`、`perf/thresholds.tsv`、`.github/workflows/perf.yml`
  - 补 Stage 2 runtime slot 场景和 stage 过滤支持。
- 不属于 `Stage 2` 的内容：
  - 不做 `Frozen/Mutable` frame 二分
  - 不做 arena 压缩
  - 不引入第二套 benchmark 框架
  - 不改变错误类型对外结构

### Profile Plan
- `Stage 2` 的 profile 分成 benchmark 和 sampling 两层。
- benchmark 复用现有 `perf` 基础设施，但 Stage 2 命令面固定要求按阶段过滤：
  - `cargo run --release --bin perf -- capture --output /tmp/base.tsv --stage symbol_interning`
  - `cargo run --release --bin perf -- bench --stage symbol_interning`
  - `cargo run --release --bin perf -- compare --base /tmp/base.tsv --head /tmp/head.tsv --thresholds perf/thresholds.tsv --markdown-out /tmp/perf-summary.md --stage symbol_interning`
- Stage 2 新增 4 组运行时场景族：
  - `slot_closure_deep_capture_{16|64|128}`
  - `slot_lookup_chain_{16|64|128}`
  - `slot_named_let_wide_{16|64|128}`
  - `slot_do_wide_{16|64|128}`
- gate 默认只用 `64` 档；`16` 和 `128` 用于 Criterion 与本地分析。
- 本地 sampling profiler 默认使用 `samply`，仅作为开发分析步骤，不进入 CI 依赖：
  - parse-heavy：`samply record -- cargo run --release --bin perf -- capture --output /tmp/symbol-parse.tsv --stage symbol_interning --filter 'parse_*'`
  - runtime-heavy：`samply record -- cargo run --release --bin perf -- capture --output /tmp/symbol-runtime.tsv --stage symbol_interning --filter 'slot_*_64'`
- 采样时要重点观察：
  - `parse_lisp_variable` / symbol parse 路径
  - `intern_symbol` / interner hash 路径
  - resolver 的 scope push/pop 与变量降级路径
  - `LocalEnv::lookup_local`
  - `process_expression_list` 的 global/local 分派路径
  - `capture_env` 的 capture spec 执行路径
- Stage 2 完成后应当显著缩小或消失的热点：
  - `HashMap<String, Value>` 相关查找
  - 基于 `String` 的 free-var 集合构造
  - `Lambda::collect_free_vars` 的字符串扫描路径
- 如果 sampling 仍显示 `named let` / `do` 热点主要在 frame 分配或 `RefLock<Vec<Value>>` 扩容，而不是 lookup，本阶段只记录结论，不提前吞并 `Stage 3` 的布局优化。

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
  - 不同 symbol 名称对应不同 `SymbolId`。
  - parser 返回的绑定位置保留为 `Expression::Symbol`，普通求值位置变成 `Expression::Variable`。
  - 嵌套 lambda 中内层 body 的外层变量解析为正确的 `depth + slot`。
  - `named let` 的递归名只在 body 降到 local slot，不在 initializer 中可见。
  - `do` 的 `init` 不看到 loop slot，`test` / `body` / `step` 看到同一 frame slot。
  - 局部 shadow builtin 时，list head 解析为 `ResolvedVar::Local`，不按 special form 处理。
  - 全局 shadow builtin 时，后续 parse 使用当前 root 状态，按普通 global call 解析。
  - `Expression::Variable(ResolvedVar::Local)` 读取正确 slot。
  - `Expression::Variable(ResolvedVar::Global)` 读取正确全局值。
  - closure 仍然是定义时快照，包括对全局值的捕获。
  - `Expression`/`Lambda` 的显示结果仍打印原始 symbol 文本。
  - 所有 `NotFoundVariable` / `UnboundFunction` / 参数错误仍输出原始文本名。
  - builtin shadowing、named `let`、`do`、高阶函数场景在 interning + slot 后行为不变。
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
- Stage 2 落地后，`capture` 与 `bench` 也必须支持 `--stage`，避免三阶段 workload 全量混跑。
- Stage 2 的 runtime profile 默认增加这组场景：
  - `slot_closure_deep_capture_{16|64|128}`
  - `slot_lookup_chain_{16|64|128}`
  - `slot_named_let_wide_{16|64|128}`
  - `slot_do_wide_{16|64|128}`
- Stage 2 gate 默认启用这些条目：
  - `parse_symbol_dense >= 25%`
  - `parse_nested_forms >= 10%`
  - `eval_shadow_builtin >= 10%`
  - `slot_lookup_chain_64 >= 20%`
  - `slot_named_let_wide_64 >= 15%`
  - `slot_do_wide_64 >= 10%`
  - `slot_closure_deep_capture_64` 最大回退 `5%`
- Stage 2 本地 profile 默认同时产出两份 artifact：
  - `perf` 的 compare markdown 和 Criterion 报表
  - `samply` 的 parse-heavy 与 runtime-heavy 采样结果

## Assumptions
- 允许新增 `criterion` 作为唯一 benchmark 框架。
- 不引入第二套 benchmark 框架。
- 不引入 `xtask`；所有性能辅助命令统一通过单个 `perf` bin 暴露。
- 第一阶段允许调整公开 `Expression::eval` / `ProcessorFunc` 相关签名，并新增公开 `LocalEnv<'gc>`。
- 第二阶段允许破坏性修改公开 parser API。
- 第二阶段默认直接落地 `Symbol<'gc> + LocalSlot<'gc>`，不把 lexical address/slot 推迟到第三阶段。
- 第二阶段默认使用 `samply` 作为本地 sampling profiler；CI 仍只依赖 `perf` bin + Criterion。
- 错误类型保持字符串输出，不把 `SymbolId` 暴露到错误层。
- Stage 1 是必须落地的详细实施方案；Stage 2 是紧随其后的确定性方案；Stage 3 只有在 profile 证明有效时才实施。
- 后续优化默认不把“链式 `HashMap<String, Value>` env”当作长期终态，而是继续向精确捕获、symbol/slot 查找和按语义分流的数据结构推进。
- 默认验证基线保持为：`cargo fmt --check`、`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings`。
