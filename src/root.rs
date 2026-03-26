//! GC arena root and environment management for the Lisp interpreter.
//!
//! This module defines the root structure that holds the global environment,
//! along with the GC arena type and token for integrating with `gc-arena`.
//! It also defines the `LocalEnv` type for efficient lexical scoping.

use gc_arena::lock::RefLock;
use gc_arena::{Arena, Gc, Mutation, Rootable};
use gc_arena_derive::Collect;
use std::collections::{HashMap, HashSet};

use crate::process::{
    addition_call, and_call, cond_call, define_call, division_call, do_call, equal_call,
    greater_equal_call, greater_than_call, if_call, lambda_call, less_equal_call, less_than_call,
    let_call, multiplication_call, or_call, subtraction_call,
};
use crate::symbol::{BuiltinSymbols, LocalSlot, ResolvedVar, Symbol, SymbolId, SymbolTable};
use crate::{errors::LispComputerError, parse::Expression, value::Value};

/// An environment frame containing variable bindings.
///
/// `EnvFrame` is an internal structure that represents a single lexical
/// frame in the environment chain. It has a parent frame (for outer scopes)
/// and a hash map of variable bindings.
#[derive(Collect, Debug, PartialEq)]
#[collect(no_drop)]
pub struct FrameLayout<'gc> {
    pub symbols: Box<[Symbol<'gc>]>,
    pub slot_by_id: HashMap<SymbolId, u16>,
    pub mutable: bool,
}

impl<'gc> FrameLayout<'gc> {
    pub fn new(symbols: Vec<Symbol<'gc>>, mutable: bool) -> Self {
        let mut slot_by_id = HashMap::new();
        for (slot, symbol) in symbols.iter().enumerate() {
            slot_by_id.insert(symbol.id, slot as u16);
        }
        Self {
            symbols: symbols.into_boxed_slice(),
            slot_by_id,
            mutable,
        }
    }
}

#[derive(Collect, Debug, PartialEq)]
#[collect(no_drop)]
pub struct EnvFrame<'gc> {
    pub parent: Option<Gc<'gc, EnvFrame<'gc>>>,
    pub layout: Gc<'gc, FrameLayout<'gc>>,
    pub values: Gc<'gc, RefLock<Vec<Value<'gc>>>>,
}

/// A handle to a local environment for lexical scoping.
///
/// `LocalEnv` is a lightweight handle that points to an environment frame
/// in the GC arena. It provides methods for environment lookup, extension,
/// and mutation. Unlike the old model that cloned entire hash maps, this
/// uses a chain of frames for efficient closure capture.
///
/// The environment lookup follows the chain: current frame -> parent frames
/// -> global environment (via LispRoot).
#[derive(Collect, Debug, Clone, PartialEq)]
#[collect(no_drop)]
pub struct LocalEnv<'gc> {
    pub frame: Option<Gc<'gc, EnvFrame<'gc>>>,
}

impl<'gc> LocalEnv<'gc> {
    /// Create an empty local environment with no bindings.
    pub fn empty() -> Self {
        LocalEnv { frame: None }
    }

    pub fn extend_slots(
        &self,
        layout: Gc<'gc, FrameLayout<'gc>>,
        values: Vec<Value<'gc>>,
        mc: &'gc Mutation<'gc>,
    ) -> Self {
        let new_frame = Gc::new(
            mc,
            EnvFrame {
                parent: self.frame,
                layout,
                values: Gc::new(mc, RefLock::new(values)),
            },
        );
        LocalEnv {
            frame: Some(new_frame),
        }
    }

    pub fn extend_bindings(
        &self,
        bindings: Vec<(Symbol<'gc>, Value<'gc>)>,
        mutable: bool,
        mc: &'gc Mutation<'gc>,
    ) -> Self {
        let symbols = bindings.iter().map(|(symbol, _)| *symbol).collect();
        let values = bindings.into_iter().map(|(_, value)| value).collect();
        let layout = Gc::new(mc, FrameLayout::new(symbols, mutable));
        self.extend_slots(layout, values, mc)
    }

    pub fn lookup_local(&self, local: LocalSlot<'gc>) -> Option<Value<'gc>> {
        let mut current = self.frame;
        for _ in 0..local.depth {
            current = current?.parent;
        }
        let frame = current?;
        frame.values.borrow().get(local.slot as usize).cloned()
    }

    pub fn lookup_symbol(&self, symbol: SymbolId) -> Option<Value<'gc>> {
        let mut current = self.frame;
        while let Some(frame) = current {
            if let Some(slot) = frame.layout.slot_by_id.get(&symbol) {
                return frame.values.borrow().get(*slot as usize).cloned();
            }
            current = frame.parent;
        }
        None
    }

    /// Insert a binding into the current frame only.
    ///
    /// This method only modifies the innermost frame (if it exists).
    /// It does not search parent frames. Used by `do` for mutable
    /// loop variables.
    pub fn set_slot_here(&self, slot: u16, value: Value<'gc>, mc: &'gc Mutation<'gc>) {
        if let Some(frame) = self.frame
            && let Some(existing) = frame.values.borrow_mut(mc).get_mut(slot as usize)
        {
            *existing = value;
        }
    }

    pub fn snapshot(&self, mc: &'gc Mutation<'gc>) -> Self {
        self.snapshot_with_tail(None, mc)
    }

    pub fn snapshot_with_tail(
        &self,
        tail: Option<Gc<'gc, EnvFrame<'gc>>>,
        mc: &'gc Mutation<'gc>,
    ) -> Self {
        fn snapshot_frame<'gc>(
            frame: Option<Gc<'gc, EnvFrame<'gc>>>,
            tail: Option<Gc<'gc, EnvFrame<'gc>>>,
            mc: &'gc Mutation<'gc>,
        ) -> Option<Gc<'gc, EnvFrame<'gc>>> {
            match frame {
                Some(frame) => {
                    let parent = snapshot_frame(frame.parent, tail, mc);
                    let values = frame.values.borrow().clone();
                    Some(Gc::new(
                        mc,
                        EnvFrame {
                            parent,
                            layout: frame.layout,
                            values: Gc::new(mc, RefLock::new(values)),
                        },
                    ))
                }
                None => tail,
            }
        }

        Self {
            frame: snapshot_frame(self.frame, tail, mc),
        }
    }
}

/// Root struct for the GC arena, holding global variables.
///
/// The `LispRoot` is the root of the garbage collected arena and contains
/// the global variable environment. It implements `Rootable` to allow the
/// GC arena to manage lifetimes correctly.
///
/// # Fields
/// - `variables`: A thread-safe, GC-managed hash map storing global variables
///
/// # Environment Model
/// Variable lookup is two-tiered:
/// 1. Local variables (from `let` bindings or lambda parameters) are passed
///    as the `variables` parameter to evaluation functions
/// 2. Global variables are stored in `LispRoot::variables`
///
/// The built-in functions and special forms are initialized in `LispRoot::new()`.
#[derive(Collect)]
#[collect(no_drop)]
pub struct LispRoot<'gc> {
    /// Global variables stored in a GC-managed, reference-counted hash map.
    pub variables: Gc<'gc, RefLock<HashMap<SymbolId, Value<'gc>>>>,
    pub symbols: Gc<'gc, RefLock<SymbolTable<'gc>>>,
    pub builtins: BuiltinSymbols<'gc>,
}

/// A token type that implements `Rootable` for any lifetime, linking to `LispRoot`.
///
/// This zero-sized marker type is used by `gc-arena` to associate the arena
/// with the root type. It implements `Rootable<'gc>` with `Root = LispRoot<'gc>`.
pub struct RootToken;

impl<'gc> Rootable<'gc> for RootToken {
    type Root = LispRoot<'gc>;
}

/// Type alias for the GC arena.
///
/// This is the main arena type used for allocating all GC-managed objects
/// in the Lisp interpreter. It is parameterized by a lifetime `'gc` representing
/// the arena's lifetime.
///
/// # Example
/// ```
/// use lisp::GcArena;
/// let arena = GcArena::new(|mc| lisp::LispRoot::new(mc));
/// ```
pub type GcArena<'gc> = Arena<RootToken>;

impl<'gc> LispRoot<'gc> {
    pub fn intern_symbol(&self, name: &str, mc: &'gc Mutation<'gc>) -> Symbol<'gc> {
        if let Some(symbol) = self.lookup_symbol(name) {
            return symbol;
        }

        let mut symbols = self.symbols.borrow_mut(mc);
        if let Some(id) = symbols.lookup(name) {
            return symbols.symbol(id).expect("symbol id should exist");
        }

        let id = SymbolId(symbols.names.len() as u32);
        let interned_name = Gc::new(mc, name.to_string());
        symbols.ids_by_name.insert(name.to_string(), id);
        symbols.names.push(interned_name);
        Symbol {
            id,
            name: interned_name,
        }
    }

    pub fn lookup_symbol(&self, name: &str) -> Option<Symbol<'gc>> {
        let symbols = self.symbols.borrow();
        let id = symbols.lookup(name)?;
        symbols.symbol(id)
    }

    pub fn get_global(&self, id: SymbolId) -> Option<Value<'gc>> {
        self.variables.borrow().get(&id).cloned()
    }

    pub fn set_global(&self, symbol: Symbol<'gc>, value: Value<'gc>, mc: &'gc Mutation<'gc>) {
        self.variables.borrow_mut(mc).insert(symbol.id, value);
    }

    /// Capture a set of free variables from the current local environment.
    ///
    /// Creates a new `LocalEnv` containing only the specified variable names,
    /// copying their current values from the given environment. This is used
    /// for closure capture, ensuring that captured values are immutable snapshots.
    ///
    /// # Arguments
    /// - `free_vars`: Set of variable names to capture
    /// - `locals`: Current local environment to capture from (borrowed)
    /// - `mc`: GC mutation context
    ///
    /// # Returns
    /// A new `LocalEnv` containing only the captured bindings.
    pub fn capture_env(
        &self,
        free_vars: &HashSet<String>,
        locals: &LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<LocalEnv<'gc>, LispComputerError> {
        let mut captured_bindings = Vec::new();
        for name in free_vars {
            let Some(symbol) = self.lookup_symbol(name) else {
                return Err(LispComputerError::NotFoundVariable(name.clone()));
            };
            let Some(value) = locals
                .lookup_symbol(symbol.id)
                .or_else(|| self.get_global(symbol.id))
            else {
                return Err(LispComputerError::NotFoundVariable(name.clone()));
            };
            captured_bindings.push((symbol, value));
        }
        Ok(LocalEnv::empty().extend_bindings(captured_bindings, false, mc))
    }

    pub fn resolve_runtime_variable(
        &self,
        variable: ResolvedVar<'gc>,
        locals: &LocalEnv<'gc>,
    ) -> Option<Value<'gc>> {
        match variable {
            ResolvedVar::Global(symbol) => locals
                .lookup_symbol(symbol.id)
                .or_else(|| self.get_global(symbol.id)),
            ResolvedVar::Local(local) => locals.lookup_local(local),
        }
    }

    pub fn snapshot_closure_env(
        &self,
        locals: &LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> LocalEnv<'gc> {
        let globals = self
            .variables
            .borrow()
            .iter()
            .filter_map(|(id, value)| {
                self.symbols
                    .borrow()
                    .symbol(*id)
                    .map(|symbol| (symbol, value.clone()))
            })
            .collect::<Vec<_>>();
        let global_tail = if globals.is_empty() {
            None
        } else {
            LocalEnv::empty().extend_bindings(globals, false, mc).frame
        };
        locals.snapshot_with_tail(global_tail, mc)
    }
    ///
    /// Looks up the symbol in the environment (local then global) and if found:
    /// - For `Value::Lambda`: calls the closure with the given arguments
    /// - For `Value::Processor`: calls the built-in function
    /// - For other values: returns `UnboundFunction` error
    ///
    /// # Arguments
    /// - `symbol`: The function name to look up
    /// - `args`: Array of argument expressions (unevaluated)
    /// - `locals`: Local variable environment chain
    /// - `mc`: GC mutation context
    ///
    /// # Returns
    /// - `Ok(Value)` on successful function call
    /// - `Err(LispComputerError::UnboundFunction)` if symbol not found or not callable
    pub fn process_variable(
        &'gc self,
        symbol: &str,
        args: &[Gc<'gc, Expression<'gc>>],
        locals: &LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some(value) = self.get_variable(symbol, locals) {
            match value {
                Value::Lambda(lambda) => {
                    return lambda.call(args, self, locals, locals, mc);
                }
                Value::Processor(proc, _symbol) => return proc(args, self, locals, mc),
                _ => {}
            }
        }
        Err(LispComputerError::UnboundFunction(symbol.to_string()))
    }

    pub fn process_symbol(
        &'gc self,
        symbol: Symbol<'gc>,
        args: &[Gc<'gc, Expression<'gc>>],
        locals: &LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        if let Some(value) = self.get_global(symbol.id) {
            match value {
                Value::Lambda(lambda) => return lambda.call(args, self, locals, locals, mc),
                Value::Processor(proc, _symbol) => return proc(args, self, locals, mc),
                _ => {}
            }
        }
        Err(LispComputerError::UnboundFunction(symbol.name.to_string()))
    }

    /// Set a global variable to a value.
    ///
    /// Inserts or updates a global variable in the root environment.
    ///
    /// # Arguments
    /// - `name`: Variable name
    /// - `value`: Value to store (GC-managed)
    /// - `mc`: GC mutation context
    pub fn set_variable(&self, name: String, value: Value<'gc>, mc: &'gc Mutation<'gc>) {
        let symbol = self.intern_symbol(&name, mc);
        self.set_global(symbol, value, mc);
    }

    /// Get a variable value from the environment.
    ///
    /// First checks local `locals` chain, then falls back to global
    /// variables in `self.variables`.
    ///
    /// # Arguments
    /// - `name`: Variable name to look up
    /// - `locals`: Local variable environment chain (borrowed)
    ///
    /// # Returns
    /// - `Some(Value)` if found in either local or global scope
    /// - `None` if variable is unbound
    pub fn get_variable(&self, name: &str, locals: &LocalEnv<'gc>) -> Option<Value<'gc>> {
        let symbol = self.lookup_symbol(name)?;
        locals
            .lookup_symbol(symbol.id)
            .or_else(|| self.get_global(symbol.id))
    }

    /// Create a new `LispRoot` with all built-in functions and special forms.
    ///
    /// Initializes the global environment with:
    /// - Boolean constants: `#t` (true) and `#f` (false)
    /// - Arithmetic operators: `+`, `-`, `*`, `/`
    /// - Comparison operators: `=`, `>`, `<`, `>=`, `<=`
    /// - Logical operators: `and`, `or`
    /// - Special forms: `if`, `cond`, `lambda`, `define`, `let`, `do`
    ///
    /// # Arguments
    /// - `mc`: GC mutation context for allocating global variable storage
    ///
    /// # Returns
    /// A new `LispRoot` with initialized global environment.
    pub fn new(mc: &'gc Mutation<'gc>) -> Self {
        let symbols = Gc::new(mc, RefLock::new(SymbolTable::new()));
        let mut root = LispRoot {
            variables: Gc::new(mc, RefLock::new(HashMap::new())),
            symbols,
            builtins: BuiltinSymbols {
                if_: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                cond: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                lambda: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                define: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                let_: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                do_: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                and_: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                or_: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                add: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                sub: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                mul: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                div: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                eq: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                gt: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                lt: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                ge: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                le: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                true_: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
                false_: Symbol {
                    id: SymbolId(0),
                    name: Gc::new(mc, String::new()),
                },
            },
        };

        let false_ = root.intern_symbol("#f", mc);
        let true_ = root.intern_symbol("#t", mc);
        let add = root.intern_symbol("+", mc);
        let sub = root.intern_symbol("-", mc);
        let mul = root.intern_symbol("*", mc);
        let div = root.intern_symbol("/", mc);
        let eq = root.intern_symbol("=", mc);
        let gt = root.intern_symbol(">", mc);
        let lt = root.intern_symbol("<", mc);
        let ge = root.intern_symbol(">=", mc);
        let le = root.intern_symbol("<=", mc);
        let if_ = root.intern_symbol("if", mc);
        let or_ = root.intern_symbol("or", mc);
        let and_ = root.intern_symbol("and", mc);
        let cond = root.intern_symbol("cond", mc);
        let define = root.intern_symbol("define", mc);
        let lambda = root.intern_symbol("lambda", mc);
        let let_ = root.intern_symbol("let", mc);
        let do_ = root.intern_symbol("do", mc);

        root.builtins = BuiltinSymbols {
            if_,
            cond,
            lambda,
            define,
            let_,
            do_,
            and_,
            or_,
            add,
            sub,
            mul,
            div,
            eq,
            gt,
            lt,
            ge,
            le,
            true_,
            false_,
        };

        root.set_global(false_, Value::Boolean(false), mc);
        root.set_global(true_, Value::Boolean(true), mc);
        root.set_global(add, Value::Processor(addition_call, add), mc);
        root.set_global(sub, Value::Processor(subtraction_call, sub), mc);
        root.set_global(mul, Value::Processor(multiplication_call, mul), mc);
        root.set_global(div, Value::Processor(division_call, div), mc);
        root.set_global(eq, Value::Processor(equal_call, eq), mc);
        root.set_global(gt, Value::Processor(greater_than_call, gt), mc);
        root.set_global(lt, Value::Processor(less_than_call, lt), mc);
        root.set_global(ge, Value::Processor(greater_equal_call, ge), mc);
        root.set_global(le, Value::Processor(less_equal_call, le), mc);
        root.set_global(if_, Value::Processor(if_call, if_), mc);
        root.set_global(or_, Value::Processor(or_call, or_), mc);
        root.set_global(and_, Value::Processor(and_call, and_), mc);
        root.set_global(cond, Value::Processor(cond_call, cond), mc);
        root.set_global(define, Value::Processor(define_call, define), mc);
        root.set_global(lambda, Value::Processor(lambda_call, lambda), mc);
        root.set_global(let_, Value::Processor(let_call, let_), mc);
        root.set_global(do_, Value::Processor(do_call, do_), mc);

        root
    }
}
