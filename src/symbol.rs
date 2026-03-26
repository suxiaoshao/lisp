use std::collections::HashMap;

use gc_arena::Gc;
use gc_arena_derive::Collect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Collect)]
#[collect(no_drop)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Collect)]
#[collect(no_drop)]
pub struct Symbol<'gc> {
    pub id: SymbolId,
    pub name: Gc<'gc, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Collect)]
#[collect(no_drop)]
pub struct LocalSlot<'gc> {
    pub symbol: Symbol<'gc>,
    pub depth: u16,
    pub slot: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Collect)]
#[collect(no_drop)]
pub enum ResolvedVar<'gc> {
    Global(Symbol<'gc>),
    Local(LocalSlot<'gc>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialForm {
    If,
    Cond,
    Lambda,
    Define,
    Let,
    Do,
    And,
    Or,
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct SymbolTable<'gc> {
    pub ids_by_name: HashMap<String, SymbolId>,
    pub names: Vec<Gc<'gc, String>>,
}

impl<'gc> SymbolTable<'gc> {
    pub fn new() -> Self {
        Self {
            ids_by_name: HashMap::new(),
            names: Vec::new(),
        }
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.ids_by_name.get(name).copied()
    }

    pub fn symbol(&self, id: SymbolId) -> Option<Symbol<'gc>> {
        self.names
            .get(id.0 as usize)
            .copied()
            .map(|name| Symbol { id, name })
    }
}

#[derive(Debug, Clone, Copy, Collect)]
#[collect(no_drop)]
pub struct BuiltinSymbols<'gc> {
    pub if_: Symbol<'gc>,
    pub cond: Symbol<'gc>,
    pub lambda: Symbol<'gc>,
    pub define: Symbol<'gc>,
    pub let_: Symbol<'gc>,
    pub do_: Symbol<'gc>,
    pub and_: Symbol<'gc>,
    pub or_: Symbol<'gc>,
    pub add: Symbol<'gc>,
    pub sub: Symbol<'gc>,
    pub mul: Symbol<'gc>,
    pub div: Symbol<'gc>,
    pub eq: Symbol<'gc>,
    pub gt: Symbol<'gc>,
    pub lt: Symbol<'gc>,
    pub ge: Symbol<'gc>,
    pub le: Symbol<'gc>,
    pub true_: Symbol<'gc>,
    pub false_: Symbol<'gc>,
}

impl<'gc> BuiltinSymbols<'gc> {
    pub fn special_form(&self, id: SymbolId) -> Option<SpecialForm> {
        match id {
            id if id == self.if_.id => Some(SpecialForm::If),
            id if id == self.cond.id => Some(SpecialForm::Cond),
            id if id == self.lambda.id => Some(SpecialForm::Lambda),
            id if id == self.define.id => Some(SpecialForm::Define),
            id if id == self.let_.id => Some(SpecialForm::Let),
            id if id == self.do_.id => Some(SpecialForm::Do),
            id if id == self.and_.id => Some(SpecialForm::And),
            id if id == self.or_.id => Some(SpecialForm::Or),
            _ => None,
        }
    }

    pub fn is_builtin_id(&self, id: SymbolId) -> bool {
        [
            self.if_.id,
            self.cond.id,
            self.lambda.id,
            self.define.id,
            self.let_.id,
            self.do_.id,
            self.and_.id,
            self.or_.id,
            self.add.id,
            self.sub.id,
            self.mul.id,
            self.div.id,
            self.eq.id,
            self.gt.id,
            self.lt.id,
            self.ge.id,
            self.le.id,
            self.true_.id,
            self.false_.id,
        ]
        .contains(&id)
    }
}
