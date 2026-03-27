use lisp_core::{GcArena, LispComputerError, LispRoot, eval_program, parse_program};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Parse,
    Eval,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Expect {
    ParseOk,
    ParseError,
    Value,
    RuntimeError,
}

#[derive(Debug, Clone, Deserialize)]
struct CaseManifest {
    mode: Mode,
    expect: Expect,
    result: Option<String>,
    error_contains: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    id: String,
    manifest: CaseManifest,
    program: String,
}

pub fn load_cases(root: &Path) -> Result<Vec<TestCase>, String> {
    let mut cases = Vec::new();
    collect_cases(root, root, &mut cases)?;
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(cases)
}

pub fn run_case(case: &TestCase) -> Result<(), String> {
    match (&case.manifest.mode, &case.manifest.expect) {
        (Mode::Parse, Expect::ParseOk) => parse_full_program(&case.program)
            .map_err(|err| format!("expected parse success, got error: {err}")),
        (Mode::Parse, Expect::ParseError) => {
            if parse_full_program(&case.program).is_err() {
                Ok(())
            } else {
                Err("expected parse failure, but parse succeeded".to_string())
            }
        }
        (Mode::Eval, Expect::Value) => match eval_program(&case.program) {
            Ok(result) => {
                let expected =
                    case.manifest.result.as_deref().ok_or_else(|| {
                        "missing `result` field for value expectation".to_string()
                    })?;
                if result == expected {
                    Ok(())
                } else {
                    Err(format!("expected value `{expected}`, got `{result}`"))
                }
            }
            Err(err) => Err(format!("expected value, got runtime error: {err}")),
        },
        (Mode::Eval, Expect::RuntimeError) => match eval_program(&case.program) {
            Ok(result) => Err(format!("expected runtime error, got value `{result}`")),
            Err(err) => {
                let rendered = err.to_string();
                let needle = case.manifest.error_contains.as_deref().ok_or_else(|| {
                    "missing `error_contains` field for runtime_error expectation".to_string()
                })?;
                if rendered.contains(needle) {
                    Ok(())
                } else {
                    Err(format!(
                        "expected runtime error containing `{needle}`, got `{rendered}`"
                    ))
                }
            }
        },
        (mode, expect) => Err(format!("unsupported combination: {mode:?} + {expect:?}")),
    }
}

fn collect_cases(root: &Path, dir: &Path, cases: &mut Vec<TestCase>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join("case.toml");
            let program_path = path.join("program.lisp");
            if manifest_path.exists() && program_path.exists() {
                let manifest_str =
                    fs::read_to_string(&manifest_path).map_err(|err| err.to_string())?;
                let manifest: CaseManifest =
                    toml::from_str(&manifest_str).map_err(|err| err.to_string())?;
                let program = fs::read_to_string(&program_path).map_err(|err| err.to_string())?;
                let relative = path
                    .strip_prefix(root)
                    .map_err(|err| err.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                cases.push(TestCase {
                    id: relative,
                    manifest,
                    program,
                });
            } else {
                collect_cases(root, &path, cases)?;
            }
        }
    }
    Ok(())
}

fn parse_full_program(input: &str) -> Result<(), LispComputerError> {
    let arena = GcArena::new(|mc| LispRoot::new(mc));
    arena.mutate(|mc, _root| {
        let (remaining, _expressions) = parse_program(mc, input)
            .map_err(|_| LispComputerError::InvalidExpression("parse error".to_string()))?;
        if remaining.trim().is_empty() {
            Ok(())
        } else {
            Err(LispComputerError::InvalidExpression(format!(
                "unparsed input: {}",
                remaining.trim()
            )))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn bundled_cases_pass() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cases");
        let cases = load_cases(&root).expect("load cases");
        assert!(!cases.is_empty(), "expected bundled conformance cases");

        for case in cases {
            if let Err(err) = run_case(&case) {
                panic!("case {} failed: {}", case.id, err);
            }
        }
    }
}
