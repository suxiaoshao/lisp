//! Lisp expression parser using nom combinators.

pub mod string;

use std::fmt::Display;

use crate::{
    Symbol, SymbolId,
    errors::LispComputerError,
    process::process_expression_list,
    root::LispRoot,
    symbol::{LocalSlot, ResolvedVar, SpecialForm},
    value::Value,
};
use gc_arena::{Gc, Mutation};
use gc_arena_derive::Collect;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{multispace0, multispace1, none_of, one_of},
    combinator::{map, not, peek, recognize},
    error::Error,
    multi::{many1, separated_list0},
    number::complete::double,
    sequence::delimited,
};

#[derive(Debug, PartialEq, Clone, Collect)]
#[collect(no_drop)]
pub enum Expression<'gc> {
    Number(f64),
    Symbol(Symbol<'gc>),
    Variable(ResolvedVar<'gc>),
    List(Vec<Gc<'gc, Expression<'gc>>>),
    String(Gc<'gc, String>),
}

impl<'gc> Display for Expression<'gc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Number(number) => write!(f, "{}", number),
            Expression::Symbol(symbol) => write!(f, "{}", &*symbol.name),
            Expression::Variable(ResolvedVar::Global(symbol)) => write!(f, "{}", &*symbol.name),
            Expression::Variable(ResolvedVar::Local(local)) => write!(f, "{}", &*local.symbol.name),
            Expression::List(expressions) => write!(
                f,
                "({})",
                expressions
                    .iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Expression::String(s) => write!(f, "\"{}\"", &**s),
        }
    }
}

impl<'gc> Expression<'gc> {
    pub fn eval(
        &self,
        env: &'gc LispRoot<'gc>,
        locals: &crate::root::LocalEnv<'gc>,
        mc: &'gc Mutation<'gc>,
    ) -> Result<Value<'gc>, LispComputerError> {
        match self {
            Expression::Number(data) => Ok(Value::Number(*data)),
            Expression::Symbol(symbol) => Err(LispComputerError::InvalidExpression(format!(
                "{} is not a valid evaluable expression",
                &*symbol.name
            ))),
            Expression::Variable(variable) => env
                .resolve_runtime_variable(*variable, locals)
                .ok_or_else(|| {
                    let name = match variable {
                        ResolvedVar::Global(symbol) => symbol.name.to_string(),
                        ResolvedVar::Local(local) => local.symbol.name.to_string(),
                    };
                    LispComputerError::NotFoundVariable(name)
                }),
            Expression::List(expressions) => process_expression_list(expressions, env, locals, mc),
            Expression::String(s) => Ok(Value::String(*s)),
        }
    }
}

pub fn parse_expression<'i, 'gc>(
    mc: &'gc Mutation<'gc>,
    root: &'gc LispRoot<'gc>,
    input: &'i str,
) -> IResult<&'i str, Gc<'gc, Expression<'gc>>> {
    let (input, expr) = parse_raw_expression(mc, root, input)?;
    let resolved = Resolver::new(root, mc).resolve_expr(expr);
    Ok((input, resolved))
}

fn parse_raw_expression<'i, 'gc>(
    mc: &'gc Mutation<'gc>,
    root: &'gc LispRoot<'gc>,
    input: &'i str,
) -> IResult<&'i str, Gc<'gc, Expression<'gc>>> {
    let (input, data) = alt((
        map(double, |n| Gc::new(mc, Expression::Number(n))),
        map(
            (
                tag("("),
                |i| parse_raw_expression_inner(mc, root, i),
                tag(")"),
            ),
            |(_, data, _)| Gc::new(mc, Expression::List(data)),
        ),
        map(parse_lisp_variable, |name| {
            Gc::new(mc, Expression::Symbol(root.intern_symbol(&name, mc)))
        }),
        map(string::parse_string::<Error<&str>>, |s| {
            Gc::new(mc, Expression::String(Gc::new(mc, s)))
        }),
    ))
    .parse(input)?;
    Ok((input, data))
}

fn parse_raw_expression_inner<'i, 'gc>(
    mc: &'gc Mutation<'gc>,
    root: &'gc LispRoot<'gc>,
    input: &'i str,
) -> IResult<&'i str, Vec<Gc<'gc, Expression<'gc>>>> {
    let (input, data) = delimited(
        multispace0,
        separated_list0(multispace1, move |i| parse_raw_expression(mc, root, i)),
        multispace0,
    )
    .parse(input)?;
    Ok((input, data))
}

fn parse_lisp_variable(input: &str) -> IResult<&str, String> {
    let valid_char = none_of(" \t\n\r()\"");
    let (input, data) =
        recognize((not(peek(one_of("0123456789"))), many1(valid_char))).parse(input)?;
    Ok((input, data.to_string()))
}

#[derive(Debug, Clone)]
struct ResolverFrame<'gc> {
    bindings: Vec<Symbol<'gc>>,
}

impl<'gc> ResolverFrame<'gc> {
    fn lookup(&self, id: SymbolId) -> Option<u16> {
        self.bindings
            .iter()
            .position(|symbol| symbol.id == id)
            .map(|idx| idx as u16)
    }
}

struct Resolver<'gc> {
    root: &'gc LispRoot<'gc>,
    mc: &'gc Mutation<'gc>,
    scopes: Vec<ResolverFrame<'gc>>,
}

impl<'gc> Resolver<'gc> {
    fn new(root: &'gc LispRoot<'gc>, mc: &'gc Mutation<'gc>) -> Self {
        Self {
            root,
            mc,
            scopes: Vec::new(),
        }
    }

    fn resolve_expr(&mut self, expr: Gc<'gc, Expression<'gc>>) -> Gc<'gc, Expression<'gc>> {
        match &*expr {
            Expression::Number(_) | Expression::String(_) | Expression::Variable(_) => expr,
            Expression::Symbol(symbol) => {
                Gc::new(self.mc, Expression::Variable(self.resolve_symbol(*symbol)))
            }
            Expression::List(exprs) => self.resolve_list(exprs),
        }
    }

    fn resolve_symbol(&self, symbol: Symbol<'gc>) -> ResolvedVar<'gc> {
        for (depth, frame) in self.scopes.iter().rev().enumerate() {
            if let Some(slot) = frame.lookup(symbol.id) {
                return ResolvedVar::Local(LocalSlot {
                    symbol,
                    depth: depth as u16,
                    slot,
                });
            }
        }
        ResolvedVar::Global(symbol)
    }

    fn resolve_list(&mut self, exprs: &[Gc<'gc, Expression<'gc>>]) -> Gc<'gc, Expression<'gc>> {
        if let Some((head, tail)) = exprs.split_first()
            && let Some(symbol) = self.as_symbol(*head)
            && !self.is_shadowed_locally(symbol.id)
            && self.active_special_form(symbol).is_some()
        {
            return self.resolve_special_list(symbol, tail);
        }

        Gc::new(
            self.mc,
            Expression::List(exprs.iter().map(|expr| self.resolve_expr(*expr)).collect()),
        )
    }

    fn as_symbol(&self, expr: Gc<'gc, Expression<'gc>>) -> Option<Symbol<'gc>> {
        match &*expr {
            Expression::Symbol(symbol) => Some(*symbol),
            _ => None,
        }
    }

    fn is_shadowed_locally(&self, id: SymbolId) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|frame| frame.lookup(id).is_some())
    }

    fn active_special_form(&self, symbol: Symbol<'gc>) -> Option<SpecialForm> {
        let special = self.root.builtins.special_form(symbol.id)?;
        match self.root.get_global(symbol.id) {
            Some(Value::Processor(_, builtin_symbol)) if builtin_symbol.id == symbol.id => {
                Some(special)
            }
            _ => None,
        }
    }

    fn resolve_special_list(
        &mut self,
        symbol: Symbol<'gc>,
        tail: &[Gc<'gc, Expression<'gc>>],
    ) -> Gc<'gc, Expression<'gc>> {
        let head = Gc::new(self.mc, Expression::Variable(ResolvedVar::Global(symbol)));
        let mut resolved = vec![head];

        match self.active_special_form(symbol) {
            Some(SpecialForm::Lambda) => self.resolve_lambda_tail(&mut resolved, tail),
            Some(SpecialForm::Define) => self.resolve_define_tail(&mut resolved, tail),
            Some(SpecialForm::Let) => self.resolve_let_tail(&mut resolved, tail),
            Some(SpecialForm::Do) => self.resolve_do_tail(&mut resolved, tail),
            Some(SpecialForm::Cond) => self.resolve_cond_tail(&mut resolved, tail),
            Some(SpecialForm::If | SpecialForm::And | SpecialForm::Or) => {
                resolved.extend(tail.iter().map(|expr| self.resolve_expr(*expr)));
            }
            None => resolved.extend(tail.iter().map(|expr| self.resolve_expr(*expr))),
        }

        Gc::new(self.mc, Expression::List(resolved))
    }

    fn resolve_lambda_tail(
        &mut self,
        resolved: &mut Vec<Gc<'gc, Expression<'gc>>>,
        tail: &[Gc<'gc, Expression<'gc>>],
    ) {
        let Some((params_gc, body)) = tail.split_first() else {
            return;
        };

        match &**params_gc {
            Expression::List(params) => {
                let param_symbols = self.binding_symbols(params);
                let params_exprs = params
                    .iter()
                    .map(|param| self.resolve_binding_expr(*param))
                    .collect();
                resolved.push(Gc::new(self.mc, Expression::List(params_exprs)));
                self.push_scope(param_symbols);
                resolved.extend(body.iter().map(|expr| self.resolve_expr(*expr)));
                self.pop_scope();
            }
            _ => {
                resolved.push(self.resolve_expr(*params_gc));
                resolved.extend(body.iter().map(|expr| self.resolve_expr(*expr)));
            }
        }
    }

    fn resolve_define_tail(
        &mut self,
        resolved: &mut Vec<Gc<'gc, Expression<'gc>>>,
        tail: &[Gc<'gc, Expression<'gc>>],
    ) {
        match tail {
            [first, second] => {
                match &**first {
                    Expression::Symbol(_) => resolved.push(self.resolve_binding_expr(*first)),
                    Expression::List(signature) => {
                        let param_symbols = self.binding_symbols(signature.get(1..).unwrap_or(&[]));
                        let signature_exprs = signature
                            .iter()
                            .map(|expr| self.resolve_binding_expr(*expr))
                            .collect();
                        resolved.push(Gc::new(self.mc, Expression::List(signature_exprs)));
                        self.push_scope(param_symbols);
                        resolved.push(self.resolve_expr(*second));
                        self.pop_scope();
                        return;
                    }
                    _ => resolved.push(self.resolve_expr(*first)),
                }
                resolved.push(self.resolve_expr(*second));
            }
            _ => resolved.extend(tail.iter().map(|expr| self.resolve_expr(*expr))),
        }
    }

    fn resolve_let_tail(
        &mut self,
        resolved: &mut Vec<Gc<'gc, Expression<'gc>>>,
        tail: &[Gc<'gc, Expression<'gc>>],
    ) {
        match tail {
            [name_expr, bindings_gc, body @ ..]
                if matches!(&**name_expr, Expression::Symbol(_)) =>
            {
                resolved.push(self.resolve_binding_expr(*name_expr));
                if let Expression::List(bindings) = &**bindings_gc {
                    let binding_symbols = self.binding_symbols_from_pairs(bindings);
                    resolved.push(Gc::new(
                        self.mc,
                        Expression::List(
                            bindings
                                .iter()
                                .map(|binding| self.resolve_let_binding(*binding))
                                .collect(),
                        ),
                    ));
                    if let Some(name_symbol) = self.as_symbol(*name_expr) {
                        self.push_scope(vec![name_symbol]);
                        self.push_scope(binding_symbols);
                        resolved.extend(body.iter().map(|expr| self.resolve_expr(*expr)));
                        self.pop_scope();
                        self.pop_scope();
                    } else {
                        resolved.extend(body.iter().map(|expr| self.resolve_expr(*expr)));
                    }
                } else {
                    resolved.push(self.resolve_expr(*bindings_gc));
                    resolved.extend(body.iter().map(|expr| self.resolve_expr(*expr)));
                }
            }
            [bindings_gc, body @ ..] => {
                if let Expression::List(bindings) = &**bindings_gc {
                    let binding_symbols = self.binding_symbols_from_pairs(bindings);
                    resolved.push(Gc::new(
                        self.mc,
                        Expression::List(
                            bindings
                                .iter()
                                .map(|binding| self.resolve_let_binding(*binding))
                                .collect(),
                        ),
                    ));
                    self.push_scope(binding_symbols);
                    resolved.extend(body.iter().map(|expr| self.resolve_expr(*expr)));
                    self.pop_scope();
                } else {
                    resolved.extend(tail.iter().map(|expr| self.resolve_expr(*expr)));
                }
            }
            _ => resolved.extend(tail.iter().map(|expr| self.resolve_expr(*expr))),
        }
    }

    fn resolve_do_tail(
        &mut self,
        resolved: &mut Vec<Gc<'gc, Expression<'gc>>>,
        tail: &[Gc<'gc, Expression<'gc>>],
    ) {
        match tail {
            [bindings_gc, test_gc, body @ ..] => {
                if let Expression::List(bindings) = &**bindings_gc {
                    let binding_symbols = self.binding_symbols_from_triples(bindings);
                    resolved.push(Gc::new(
                        self.mc,
                        Expression::List(
                            bindings
                                .iter()
                                .map(|binding| self.resolve_do_binding(*binding))
                                .collect(),
                        ),
                    ));
                    self.push_scope(binding_symbols);
                    resolved.push(self.resolve_do_test(*test_gc));
                    resolved.extend(body.iter().map(|expr| self.resolve_expr(*expr)));
                    self.pop_scope();
                } else {
                    resolved.extend(tail.iter().map(|expr| self.resolve_expr(*expr)));
                }
            }
            _ => resolved.extend(tail.iter().map(|expr| self.resolve_expr(*expr))),
        }
    }

    fn resolve_cond_tail(
        &mut self,
        resolved: &mut Vec<Gc<'gc, Expression<'gc>>>,
        tail: &[Gc<'gc, Expression<'gc>>],
    ) {
        for (idx, clause) in tail.iter().enumerate() {
            match &**clause {
                Expression::List(items) => {
                    let clause_exprs = if idx == tail.len().saturating_sub(1)
                        && matches!(items.first().map(|expr| &**expr), Some(Expression::Symbol(symbol)) if &*symbol.name == "else")
                    {
                        items
                            .iter()
                            .enumerate()
                            .map(|(item_idx, expr)| {
                                if item_idx == 0 {
                                    self.resolve_binding_expr(*expr)
                                } else {
                                    self.resolve_expr(*expr)
                                }
                            })
                            .collect()
                    } else {
                        items.iter().map(|expr| self.resolve_expr(*expr)).collect()
                    };
                    resolved.push(Gc::new(self.mc, Expression::List(clause_exprs)));
                }
                _ => resolved.push(self.resolve_expr(*clause)),
            }
        }
    }

    fn resolve_let_binding(
        &mut self,
        binding: Gc<'gc, Expression<'gc>>,
    ) -> Gc<'gc, Expression<'gc>> {
        match &*binding {
            Expression::List(items) if items.len() == 2 => {
                let first = self.resolve_binding_expr(items[0]);
                let second = self.resolve_expr(items[1]);
                Gc::new(self.mc, Expression::List(vec![first, second]))
            }
            _ => self.resolve_expr(binding),
        }
    }

    fn resolve_do_binding(
        &mut self,
        binding: Gc<'gc, Expression<'gc>>,
    ) -> Gc<'gc, Expression<'gc>> {
        match &*binding {
            Expression::List(items) if items.len() == 3 => {
                let first = self.resolve_binding_expr(items[0]);
                let second = self.resolve_expr(items[1]);
                let third = self.resolve_expr(items[2]);
                Gc::new(self.mc, Expression::List(vec![first, second, third]))
            }
            _ => self.resolve_expr(binding),
        }
    }

    fn resolve_do_test(&mut self, test: Gc<'gc, Expression<'gc>>) -> Gc<'gc, Expression<'gc>> {
        match &*test {
            Expression::List(items) => Gc::new(
                self.mc,
                Expression::List(items.iter().map(|expr| self.resolve_expr(*expr)).collect()),
            ),
            _ => self.resolve_expr(test),
        }
    }

    fn resolve_binding_expr(&self, expr: Gc<'gc, Expression<'gc>>) -> Gc<'gc, Expression<'gc>> {
        match &*expr {
            Expression::Symbol(symbol) => Gc::new(self.mc, Expression::Symbol(*symbol)),
            _ => expr,
        }
    }

    fn binding_symbols(&self, exprs: &[Gc<'gc, Expression<'gc>>]) -> Vec<Symbol<'gc>> {
        exprs
            .iter()
            .filter_map(|expr| match &**expr {
                Expression::Symbol(symbol) => Some(*symbol),
                _ => None,
            })
            .collect()
    }

    fn binding_symbols_from_pairs(
        &self,
        bindings: &[Gc<'gc, Expression<'gc>>],
    ) -> Vec<Symbol<'gc>> {
        bindings
            .iter()
            .filter_map(|binding| match &**binding {
                Expression::List(items) => match items.first().map(|expr| &**expr) {
                    Some(Expression::Symbol(symbol)) => Some(*symbol),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    fn binding_symbols_from_triples(
        &self,
        bindings: &[Gc<'gc, Expression<'gc>>],
    ) -> Vec<Symbol<'gc>> {
        self.binding_symbols_from_pairs(bindings)
    }

    fn push_scope(&mut self, bindings: Vec<Symbol<'gc>>) {
        self.scopes.push(ResolverFrame { bindings });
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{GcArena, LispComputerError, ResolvedVar};
    use anyhow::Result;

    #[test]
    fn parse_expression_inner_test() -> Result<()> {
        let arena = GcArena::new(|mc| crate::LispRoot::new(mc));
        arena.mutate(|mc, root| -> Result<(), LispComputerError> {
            let input = "1 1";
            let (remaining, exprs) = parse_raw_expression_inner(mc, root, input)
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            assert_eq!(remaining, "");
            assert_eq!(exprs.len(), 2);
            assert_eq!(&*exprs[0], &Expression::Number(1.0));
            assert_eq!(&*exprs[1], &Expression::Number(1.0));
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn parse_expression_test() -> Result<()> {
        let arena = GcArena::new(|mc| crate::LispRoot::new(mc));
        arena.mutate(|mc, root| -> Result<(), LispComputerError> {
            let input = "(+ 1 1)";
            let (remaining, expr) = parse_expression(mc, root, input)
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            assert_eq!(remaining, "");
            let Expression::List(items) = &*expr else {
                panic!("expected list");
            };
            assert!(matches!(
                &*items[0],
                Expression::Variable(ResolvedVar::Global(_))
            ));
            assert_eq!(&*items[1], &Expression::Number(1.0));
            assert_eq!(&*items[2], &Expression::Number(1.0));
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn parse_lisp_symbol_test() {
        assert_eq!(
            parse_lisp_variable("abc ").unwrap(),
            (" ", "abc".to_string())
        );
        assert!(parse_lisp_variable("123").is_err());
    }

    #[test]
    fn lambda_bindings_stay_symbols_while_body_resolves() -> Result<()> {
        let arena = GcArena::new(|mc| crate::LispRoot::new(mc));
        arena.mutate(|mc, root| -> Result<(), LispComputerError> {
            let (_, expr) = parse_expression(mc, root, "(lambda (x) x)")
                .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
            let Expression::List(items) = &*expr else {
                panic!("expected list");
            };
            let Expression::List(params) = &*items[1] else {
                panic!("expected params");
            };
            assert!(matches!(&*params[0], Expression::Symbol(_)));
            assert!(matches!(
                &*items[2],
                Expression::Variable(ResolvedVar::Local(_))
            ));
            Ok(())
        })?;
        Ok(())
    }
}
