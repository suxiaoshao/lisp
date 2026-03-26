use std::fmt::{Display, Formatter};
use std::time::Instant;

use crate::{GcArena, LispComputerError, LispRoot, LocalEnv, parse_expression};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerfStage {
    Env,
    SymbolInterning,
    Arena,
}

impl PerfStage {
    pub fn as_str(self) -> &'static str {
        match self {
            PerfStage::Env => "env",
            PerfStage::SymbolInterning => "symbol_interning",
            PerfStage::Arena => "arena",
        }
    }
}

impl Display for PerfStage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerfScenarioKind {
    ParseOnly,
    EvalFresh,
    EvalPersistent,
}

impl PerfScenarioKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PerfScenarioKind::ParseOnly => "parse_only",
            PerfScenarioKind::EvalFresh => "eval_fresh",
            PerfScenarioKind::EvalPersistent => "eval_persistent",
        }
    }
}

impl Display for PerfScenarioKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PerfScenario {
    pub id: &'static str,
    pub stage: PerfStage,
    pub kind: PerfScenarioKind,
    pub setup: &'static [&'static str],
    pub source: &'static str,
    pub warmup_iters: usize,
    pub measure_iters: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfResult {
    pub scenario_id: String,
    pub stage: PerfStage,
    pub kind: PerfScenarioKind,
    pub nanos_total: u128,
    pub iterations: usize,
}

impl PerfResult {
    pub fn nanos_per_iter(&self) -> u128 {
        let iterations = self.iterations.max(1) as u128;
        self.nanos_total / iterations
    }

    pub fn to_tsv_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            self.scenario_id,
            self.stage,
            self.kind,
            self.nanos_total,
            self.iterations,
            self.nanos_per_iter()
        )
    }
}

#[derive(Debug)]
pub enum PerfError {
    Parse(String),
    Runtime(LispComputerError),
}

impl Display for PerfError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PerfError::Parse(message) => write!(f, "parse error: {message}"),
            PerfError::Runtime(err) => Display::fmt(err, f),
        }
    }
}

impl std::error::Error for PerfError {}

const EMPTY_SETUP: &[&str] = &[];
const PERSISTENT_CAPTURE_SETUP: &[&str] =
    &["(define add-base ((lambda (base) (lambda (x) (+ base x))) 40))"];

const SCENARIOS: &[PerfScenario] = &[
    PerfScenario {
        id: "parse_symbol_dense",
        stage: PerfStage::SymbolInterning,
        kind: PerfScenarioKind::ParseOnly,
        setup: EMPTY_SETUP,
        source: "(let ((alpha 1) (beta 2) (gamma 3)) (let ((delta alpha) (epsilon beta) (zeta gamma)) (+ alpha beta gamma delta epsilon zeta alpha beta gamma delta epsilon zeta alpha beta gamma delta epsilon zeta)))",
        warmup_iters: 32,
        measure_iters: 256,
    },
    PerfScenario {
        id: "parse_nested_forms",
        stage: PerfStage::SymbolInterning,
        kind: PerfScenarioKind::ParseOnly,
        setup: EMPTY_SETUP,
        source: "(let ((x 1)) (if #t (do ((n 4 (- n 1)) (acc x (+ acc x))) ((= n 0) acc) (let ((y (+ acc x))) y)) (cond ((= x 0) x) (else (+ x 1)))))",
        warmup_iters: 32,
        measure_iters: 256,
    },
    PerfScenario {
        id: "eval_arith_deep",
        stage: PerfStage::Env,
        kind: PerfScenarioKind::EvalFresh,
        setup: EMPTY_SETUP,
        source: "(+ (* 2 3) (/ 100 5) (- 40 1) (+ 1 (* 2 (+ 3 (* 4 5)))) (* (+ 1 2) (- 9 3)))",
        warmup_iters: 16,
        measure_iters: 256,
    },
    PerfScenario {
        id: "eval_closure_chain",
        stage: PerfStage::Env,
        kind: PerfScenarioKind::EvalFresh,
        setup: EMPTY_SETUP,
        source: "((((lambda (x) (lambda (y) (lambda (z) (+ x (+ y z))))) 10) 20) 30)",
        warmup_iters: 16,
        measure_iters: 256,
    },
    PerfScenario {
        id: "eval_named_let_loop",
        stage: PerfStage::Env,
        kind: PerfScenarioKind::EvalFresh,
        setup: EMPTY_SETUP,
        source: "(let loop ((n 50) (acc 0)) (if (= n 0) acc (loop (- n 1) (+ acc n))))",
        warmup_iters: 16,
        measure_iters: 128,
    },
    PerfScenario {
        id: "eval_do_loop",
        stage: PerfStage::Env,
        kind: PerfScenarioKind::EvalFresh,
        setup: EMPTY_SETUP,
        source: "(do ((n 50 (- n 1)) (acc 0 (+ acc n))) ((= n 0) acc))",
        warmup_iters: 16,
        measure_iters: 128,
    },
    PerfScenario {
        id: "eval_shadow_builtin",
        stage: PerfStage::SymbolInterning,
        kind: PerfScenarioKind::EvalFresh,
        setup: EMPTY_SETUP,
        source: "((let ((+ (lambda (x y) 42))) (lambda () (+ 1 2))))",
        warmup_iters: 16,
        measure_iters: 256,
    },
    PerfScenario {
        id: "eval_persistent_global_capture",
        stage: PerfStage::Arena,
        kind: PerfScenarioKind::EvalPersistent,
        setup: PERSISTENT_CAPTURE_SETUP,
        source: "(add-base 2)",
        warmup_iters: 32,
        measure_iters: 512,
    },
];

pub fn all_scenarios() -> &'static [PerfScenario] {
    SCENARIOS
}

pub fn capture_scenario(scenario: &PerfScenario) -> Result<PerfResult, PerfError> {
    for _ in 0..scenario.warmup_iters {
        run_scenario_once(scenario)?;
    }

    let start = Instant::now();
    for _ in 0..scenario.measure_iters {
        run_scenario_once(scenario)?;
    }

    Ok(PerfResult {
        scenario_id: scenario.id.to_string(),
        stage: scenario.stage,
        kind: scenario.kind,
        nanos_total: start.elapsed().as_nanos(),
        iterations: scenario.measure_iters,
    })
}

pub fn run_scenario_once(scenario: &PerfScenario) -> Result<(), PerfError> {
    match scenario.kind {
        PerfScenarioKind::ParseOnly => run_parse_scenario(scenario),
        PerfScenarioKind::EvalFresh => run_eval_fresh_scenario(scenario),
        PerfScenarioKind::EvalPersistent => run_eval_persistent_scenario(scenario),
    }
}

pub fn run_parse_scenario(scenario: &PerfScenario) -> Result<(), PerfError> {
    let arena = GcArena::new(|mc| LispRoot::new(mc));
    arena.mutate(|mc, _root| -> Result<(), PerfError> {
        let (remaining, _expr) = parse_expression(mc, scenario.source)
            .map_err(|_| PerfError::Parse(scenario.id.to_string()))?;
        if !remaining.is_empty() {
            return Err(PerfError::Parse(format!(
                "{} left trailing input: {remaining}",
                scenario.id
            )));
        }
        Ok(())
    })
}

pub fn run_eval_fresh_scenario(scenario: &PerfScenario) -> Result<(), PerfError> {
    let arena = GcArena::new(|mc| LispRoot::new(mc));
    arena.mutate(|mc, root| -> Result<(), PerfError> {
        for expr in scenario.setup {
            let _ = eval_expression(mc, root, expr)?;
        }
        let _ = eval_expression(mc, root, scenario.source)?;
        Ok(())
    })
}

pub fn run_eval_persistent_scenario(scenario: &PerfScenario) -> Result<(), PerfError> {
    let arena = GcArena::new(|mc| LispRoot::new(mc));
    arena.mutate(|mc, root| -> Result<(), PerfError> {
        for expr in scenario.setup {
            let _ = eval_expression(mc, root, expr)?;
        }
        Ok(())
    })?;

    arena.mutate(|mc, root| -> Result<(), PerfError> {
        let _ = eval_expression(mc, root, scenario.source)?;
        Ok(())
    })
}

fn eval_expression<'gc>(
    mc: &'gc gc_arena::Mutation<'gc>,
    root: &'gc LispRoot<'gc>,
    input: &str,
) -> Result<String, PerfError> {
    let (remaining, expr) =
        parse_expression(mc, input).map_err(|_| PerfError::Parse(input.to_string()))?;
    if !remaining.is_empty() {
        return Err(PerfError::Parse(format!(
            "expression left trailing input: {remaining}"
        )));
    }
    let locals = LocalEnv::empty();
    let value = expr.eval(root, &locals, mc).map_err(PerfError::Runtime)?;
    Ok(format!("{value}"))
}
