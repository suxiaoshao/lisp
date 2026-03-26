# Lisp 性能优化路线图与第一阶段详细规格

## Summary
- 先做 `env` 重构，再做 `symbol interning`，最后按 profile 决定是否做 arena/布局压缩。
- 第一阶段的目标不是“局部微调”，而是把局部环境从“整表 clone”改成“链式 frame + 精确捕获”的模型，为第二阶段的 interned symbol 铺路。
- 语义保持不变：闭包仍是词法作用域、捕获仍是定义时快照、named `let` 的递归绑定仍只在函数体可见、builtin shadowing 行为不变。
- 第二阶段允许调整公开 parser API，直接把 interner 接入 parse 路径，不做兼容包装层。
- 第三阶段是 profile-gated：只有在 env + interning 完成后仍能证明分配/扫描是热点时才做。

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

## Assumptions
- 不新增第三方依赖，尤其不引入 benchmark 或 interning 库。
- 第一阶段允许调整公开 `Expression::eval` / `ProcessorFunc` 相关签名，并新增公开 `LocalEnv<'gc>`。
- 第二阶段允许破坏性修改公开 parser API。
- 错误类型保持字符串输出，不把 `SymbolId` 暴露到错误层。
- Stage 1 是必须落地的详细实施方案；Stage 2 是紧随其后的确定性方案；Stage 3 只有在 profile 证明有效时才实施。
- 默认验证基线保持为：`cargo fmt --check`、`cargo test`、`cargo clippy --all-targets --all-features -- -D warnings`。
