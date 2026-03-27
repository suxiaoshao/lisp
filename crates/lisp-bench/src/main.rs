use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Deserialize)]
struct BenchManifest {
    iterations: u32,
    warmup_iterations: u32,
}

#[derive(Debug, Clone)]
struct BenchCase {
    id: String,
    dir: PathBuf,
    manifest: BenchManifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Lisp,
    Guile,
    Racket,
}

#[derive(Debug, Serialize)]
struct BenchResult {
    case: String,
    target: String,
    status: String,
    iterations: u32,
    warmup_iterations: u32,
    median_ms: Option<f64>,
    min_ms: Option<f64>,
    max_ms: Option<f64>,
    reason: Option<String>,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let json_only = args.iter().any(|arg| arg == "--json");
    let targets = parse_targets(&args)?;
    let case_filter = parse_case_filter(&args);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cases");
    let mut cases = load_cases(&root)?;
    if let Some(case_filter) = case_filter.as_deref() {
        cases.retain(|case| case.id == case_filter);
    }

    if cases.is_empty() {
        return Err("no benchmark cases selected".to_string());
    }

    let mut results = Vec::new();
    for case in &cases {
        for target in &targets {
            results.push(run_case(case, *target));
        }
    }

    if json_only {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).map_err(|err| err.to_string())?
        );
    } else {
        for result in &results {
            match result.status.as_str() {
                "ok" => println!(
                    "{} [{}] median={:.3}ms min={:.3}ms max={:.3}ms",
                    result.case,
                    result.target,
                    result.median_ms.unwrap_or_default(),
                    result.min_ms.unwrap_or_default(),
                    result.max_ms.unwrap_or_default()
                ),
                _ => println!(
                    "{} [{}] {}{}",
                    result.case,
                    result.target,
                    result.status,
                    result
                        .reason
                        .as_deref()
                        .map(|reason| format!(": {reason}"))
                        .unwrap_or_default()
                ),
            }
        }
    }

    Ok(())
}

fn parse_targets(args: &[String]) -> Result<Vec<Target>, String> {
    let mut values = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--target" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--target requires a value".to_string())?;
            values = Some(value.clone());
            index += 1;
        }
        index += 1;
    }

    let raw = values.unwrap_or_else(|| "lisp,guile,racket".to_string());
    raw.split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| match part.trim() {
            "lisp" => Ok(Target::Lisp),
            "guile" => Ok(Target::Guile),
            "racket" => Ok(Target::Racket),
            other => Err(format!("unknown target `{other}`")),
        })
        .collect()
}

fn parse_case_filter(args: &[String]) -> Option<String> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--case" {
            return args.get(index + 1).cloned();
        }
        index += 1;
    }
    None
}

fn load_cases(root: &Path) -> Result<Vec<BenchCase>, String> {
    let mut cases = Vec::new();
    collect_cases(root, root, &mut cases)?;
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(cases)
}

fn collect_cases(root: &Path, dir: &Path, cases: &mut Vec<BenchCase>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join("manifest.toml");
            if manifest_path.exists() {
                let manifest_str =
                    fs::read_to_string(&manifest_path).map_err(|err| err.to_string())?;
                let manifest: BenchManifest =
                    toml::from_str(&manifest_str).map_err(|err| err.to_string())?;
                let id = path
                    .strip_prefix(root)
                    .map_err(|err| err.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                cases.push(BenchCase {
                    id,
                    dir: path,
                    manifest,
                });
            } else {
                collect_cases(root, &path, cases)?;
            }
        }
    }
    Ok(())
}

fn run_case(case: &BenchCase, target: Target) -> BenchResult {
    let target_name = target.name().to_string();
    if case.manifest.iterations == 0 {
        return BenchResult {
            case: case.id.clone(),
            target: target_name,
            status: "error".to_string(),
            iterations: case.manifest.iterations,
            warmup_iterations: case.manifest.warmup_iterations,
            median_ms: None,
            min_ms: None,
            max_ms: None,
            reason: Some(
                "invalid benchmark manifest: iterations must be greater than 0".to_string(),
            ),
        };
    }

    let mut samples = Vec::new();

    if let Err(reason) = warmup_case(case, target) {
        return BenchResult {
            case: case.id.clone(),
            target: target_name,
            status: "skip".to_string(),
            iterations: case.manifest.iterations,
            warmup_iterations: case.manifest.warmup_iterations,
            median_ms: None,
            min_ms: None,
            max_ms: None,
            reason: Some(reason),
        };
    }

    for _ in 0..case.manifest.iterations {
        let started = Instant::now();
        if let Err(reason) = execute_case(case, target) {
            return BenchResult {
                case: case.id.clone(),
                target: target_name,
                status: "error".to_string(),
                iterations: case.manifest.iterations,
                warmup_iterations: case.manifest.warmup_iterations,
                median_ms: None,
                min_ms: None,
                max_ms: None,
                reason: Some(reason),
            };
        }
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }

    samples.sort_by(|left, right| left.total_cmp(right));
    let median = samples[samples.len() / 2];
    let min = samples[0];
    let max = samples[samples.len() - 1];

    BenchResult {
        case: case.id.clone(),
        target: target_name,
        status: "ok".to_string(),
        iterations: case.manifest.iterations,
        warmup_iterations: case.manifest.warmup_iterations,
        median_ms: Some(median),
        min_ms: Some(min),
        max_ms: Some(max),
        reason: None,
    }
}

fn warmup_case(case: &BenchCase, target: Target) -> Result<(), String> {
    for _ in 0..case.manifest.warmup_iterations {
        execute_case(case, target)?;
    }
    Ok(())
}

fn execute_case(case: &BenchCase, target: Target) -> Result<(), String> {
    match target {
        Target::Lisp => {
            let source =
                fs::read_to_string(case.dir.join("lisp.lisp")).map_err(|err| err.to_string())?;
            lisp_core::eval_program(&source)
                .map(|_| ())
                .map_err(|err| err.to_string())
        }
        Target::Guile => run_command_target(
            env::var("LISP_BENCH_GUILE").unwrap_or_else(|_| "guile".to_string()),
            &["-s"],
            &case.dir.join("guile.scm"),
        ),
        Target::Racket => run_command_target(
            env::var("LISP_BENCH_RACKET").unwrap_or_else(|_| "racket".to_string()),
            &[],
            &case.dir.join("racket.rkt"),
        ),
    }
}

fn run_command_target(program: String, args: &[&str], script: &Path) -> Result<(), String> {
    if !script.exists() {
        return Err(format!("missing script {}", script.display()));
    }

    let status = Command::new(&program)
        .args(args)
        .arg(script)
        .status()
        .map_err(|err| format!("failed to start `{program}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`{program}` exited with status {status}"))
    }
}

impl Target {
    fn name(self) -> &'static str {
        match self {
            Target::Lisp => "lisp",
            Target::Guile => "guile",
            Target::Racket => "racket",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_iterations_returns_configuration_error() {
        let case = BenchCase {
            id: "invalid/zero-iterations".to_string(),
            dir: PathBuf::from("."),
            manifest: BenchManifest {
                iterations: 0,
                warmup_iterations: 0,
            },
        };

        let result = run_case(&case, Target::Lisp);
        assert_eq!(result.status, "error");
        assert_eq!(result.median_ms, None);
        assert_eq!(
            result.reason.as_deref(),
            Some("invalid benchmark manifest: iterations must be greater than 0")
        );
    }
}
