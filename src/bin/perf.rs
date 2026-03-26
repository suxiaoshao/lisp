use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use lisp::perf_support::{
    PerfResult, PerfScenario, PerfScenarioKind, PerfStage, all_scenarios, capture_scenario,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };

    match command {
        "list" => {
            for scenario in all_scenarios() {
                println!(
                    "{}\t{}\t{}",
                    scenario.id,
                    scenario.stage.as_str(),
                    scenario.kind.as_str()
                );
            }
            Ok(())
        }
        "capture" => capture_command(&args[1..]),
        "compare" => compare_command(&args[1..]),
        "bench" => bench_command(&args[1..]),
        "ci" => ci_command(&args[1..]),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: cargo run --release --bin perf -- <list|capture|compare|bench|ci> ...".to_string()
}

fn capture_command(args: &[String]) -> Result<(), String> {
    let mut output = None;
    let mut filter = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--output" => {
                index += 1;
                output = args.get(index).cloned();
            }
            "--filter" => {
                index += 1;
                filter = args.get(index).cloned();
            }
            other => return Err(format!("unknown capture argument: {other}")),
        }
        index += 1;
    }

    let output = output.ok_or_else(|| "capture requires --output <path>".to_string())?;
    let scenarios = filtered_scenarios(filter.as_deref())?;
    let mut lines =
        vec!["scenario_id\tstage\tkind\tnanos_total\titerations\tnanos_per_iter".to_string()];

    for scenario in scenarios {
        let result = capture_scenario(scenario).map_err(|err| err.to_string())?;
        lines.push(result.to_tsv_line());
    }

    fs::write(output, format!("{}\n", lines.join("\n"))).map_err(|err| err.to_string())
}

fn compare_command(args: &[String]) -> Result<(), String> {
    let mut base = None;
    let mut head = None;
    let mut thresholds = None;
    let mut markdown_out = None;
    let mut stages = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--base" => {
                index += 1;
                base = args.get(index).cloned();
            }
            "--head" => {
                index += 1;
                head = args.get(index).cloned();
            }
            "--thresholds" => {
                index += 1;
                thresholds = args.get(index).cloned();
            }
            "--markdown-out" => {
                index += 1;
                markdown_out = args.get(index).cloned();
            }
            "--stage" => {
                index += 1;
                stages.push(
                    args.get(index)
                        .cloned()
                        .ok_or_else(|| "compare requires a value after --stage".to_string())?,
                );
            }
            other => return Err(format!("unknown compare argument: {other}")),
        }
        index += 1;
    }

    let base = base.ok_or_else(|| "compare requires --base <path>".to_string())?;
    let head = head.ok_or_else(|| "compare requires --head <path>".to_string())?;
    let thresholds =
        thresholds.ok_or_else(|| "compare requires --thresholds <path>".to_string())?;
    let markdown_out =
        markdown_out.ok_or_else(|| "compare requires --markdown-out <path>".to_string())?;

    let base_results = parse_results(&fs::read_to_string(base).map_err(|err| err.to_string())?)?;
    let head_results = parse_results(&fs::read_to_string(head).map_err(|err| err.to_string())?)?;
    let thresholds =
        parse_thresholds(&fs::read_to_string(thresholds).map_err(|err| err.to_string())?)?;

    let comparison = compare_results(&base_results, &head_results, &thresholds, &stages)?;
    fs::write(markdown_out, comparison.markdown()).map_err(|err| err.to_string())?;

    if comparison.failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "performance gate failed for: {}",
            comparison.failures.join(", ")
        ))
    }
}

fn bench_command(args: &[String]) -> Result<(), String> {
    let filter = if args.len() == 2 && args[0] == "--filter" {
        Some(args[1].clone())
    } else if args.is_empty() {
        None
    } else {
        return Err("bench accepts only optional --filter <pattern>".to_string());
    };

    let mut command = Command::new("cargo");
    command.args(["bench", "--bench", "perf_eval"]);
    if let Some(filter) = filter {
        command.arg("--").arg(filter);
    }

    let status = command.status().map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("cargo bench failed".to_string())
    }
}

fn ci_command(args: &[String]) -> Result<(), String> {
    let mut base = None;
    let mut head = None;
    let mut artifacts = None;
    let mut stage = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--base" => {
                index += 1;
                base = args.get(index).cloned();
            }
            "--head" => {
                index += 1;
                head = args.get(index).cloned();
            }
            "--artifacts" => {
                index += 1;
                artifacts = args.get(index).cloned();
            }
            "--stage" => {
                index += 1;
                stage = args.get(index).cloned();
            }
            other => return Err(format!("unknown ci argument: {other}")),
        }
        index += 1;
    }

    let base = base.ok_or_else(|| "ci requires --base <path>".to_string())?;
    let head = head.ok_or_else(|| "ci requires --head <path>".to_string())?;
    let artifacts = artifacts.ok_or_else(|| "ci requires --artifacts <dir>".to_string())?;
    let stage = stage.ok_or_else(|| "ci requires --stage <stage>".to_string())?;
    let summary_path = Path::new(&artifacts).join("perf-summary.md");
    fs::create_dir_all(&artifacts).map_err(|err| err.to_string())?;

    let compare_args = vec![
        "--base".to_string(),
        base,
        "--head".to_string(),
        head,
        "--thresholds".to_string(),
        "perf/thresholds.tsv".to_string(),
        "--markdown-out".to_string(),
        summary_path.display().to_string(),
        "--stage".to_string(),
        stage,
    ];
    compare_command(&compare_args)?;
    bench_command(&[])
}

fn filtered_scenarios(filter: Option<&str>) -> Result<Vec<&'static PerfScenario>, String> {
    let scenarios: Vec<&'static PerfScenario> = all_scenarios()
        .iter()
        .filter(|scenario| match filter {
            Some(filter) => glob_matches(filter, scenario.id),
            None => true,
        })
        .collect();

    if scenarios.is_empty() {
        Err("no scenarios matched filter".to_string())
    } else {
        Ok(scenarios)
    }
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return value.starts_with(prefix) && value.ends_with(suffix);
    }
    value.contains(pattern)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Threshold {
    scenario_id: String,
    stage: String,
    min_improve_pct: i64,
    max_regress_pct: i64,
    required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComparisonRow {
    scenario_id: String,
    stage: String,
    base_nanos_per_iter: u128,
    head_nanos_per_iter: u128,
    delta_pct: i64,
    passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComparisonReport {
    rows: Vec<ComparisonRow>,
    failures: Vec<String>,
}

impl ComparisonReport {
    fn markdown(&self) -> String {
        let mut lines = vec![
            "# Performance Summary".to_string(),
            String::new(),
            "| Scenario | Stage | Base ns/iter | Head ns/iter | Delta % | Status |".to_string(),
            "| --- | --- | ---: | ---: | ---: | --- |".to_string(),
        ];

        for row in &self.rows {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} |",
                row.scenario_id,
                row.stage,
                row.base_nanos_per_iter,
                row.head_nanos_per_iter,
                row.delta_pct,
                if row.passed { "pass" } else { "fail" }
            ));
        }

        if self.failures.is_empty() {
            lines.push(String::new());
            lines.push("All required performance gates passed.".to_string());
        } else {
            lines.push(String::new());
            lines.push(format!("Failed scenarios: {}", self.failures.join(", ")));
        }

        format!("{}\n", lines.join("\n"))
    }
}

fn parse_results(input: &str) -> Result<HashMap<String, PerfResult>, String> {
    let mut results = HashMap::new();

    for (index, line) in input.lines().enumerate() {
        if index == 0 {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        let columns: Vec<&str> = line.split('\t').collect();
        if columns.len() != 6 {
            return Err(format!("invalid result line: {line}"));
        }

        let stage = parse_stage(columns[1])?;
        let kind = parse_kind(columns[2])?;
        let nanos_total = columns[3]
            .parse::<u128>()
            .map_err(|err| format!("invalid nanos_total: {err}"))?;
        let iterations = columns[4]
            .parse::<usize>()
            .map_err(|err| format!("invalid iterations: {err}"))?;

        results.insert(
            columns[0].to_string(),
            PerfResult {
                scenario_id: columns[0].to_string(),
                stage,
                kind,
                nanos_total,
                iterations,
            },
        );
    }

    Ok(results)
}

fn parse_thresholds(input: &str) -> Result<Vec<Threshold>, String> {
    let mut thresholds = Vec::new();

    for (index, line) in input.lines().enumerate() {
        if index == 0 || line.trim().is_empty() {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        if columns.len() != 5 {
            return Err(format!("invalid threshold line: {line}"));
        }

        thresholds.push(Threshold {
            scenario_id: columns[0].to_string(),
            stage: columns[1].to_string(),
            min_improve_pct: columns[2]
                .parse::<i64>()
                .map_err(|err| format!("invalid min_improve_pct: {err}"))?,
            max_regress_pct: columns[3]
                .parse::<i64>()
                .map_err(|err| format!("invalid max_regress_pct: {err}"))?,
            required: columns[4]
                .parse::<bool>()
                .map_err(|err| format!("invalid required flag: {err}"))?,
        });
    }

    Ok(thresholds)
}

fn compare_results(
    base: &HashMap<String, PerfResult>,
    head: &HashMap<String, PerfResult>,
    thresholds: &[Threshold],
    selected_stages: &[String],
) -> Result<ComparisonReport, String> {
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    let stage_filter_enabled = !selected_stages.is_empty();

    for threshold in thresholds {
        if stage_filter_enabled
            && !selected_stages
                .iter()
                .any(|stage| stage == &threshold.stage)
        {
            continue;
        }
        let Some(base_result) = base.get(&threshold.scenario_id) else {
            if threshold.required {
                failures.push(threshold.scenario_id.clone());
            }
            continue;
        };
        let Some(head_result) = head.get(&threshold.scenario_id) else {
            if threshold.required {
                failures.push(threshold.scenario_id.clone());
            }
            continue;
        };

        let base_ns = base_result.nanos_per_iter();
        let head_ns = head_result.nanos_per_iter();
        if base_ns == 0 {
            return Err(format!(
                "base nanos_per_iter is zero for {}",
                threshold.scenario_id
            ));
        }

        let delta = ((base_ns as f64 - head_ns as f64) / base_ns as f64) * 100.0;
        let delta_pct = delta.round() as i64;
        let passed =
            delta_pct >= threshold.min_improve_pct && delta_pct >= -threshold.max_regress_pct;

        if !passed && threshold.required {
            failures.push(threshold.scenario_id.clone());
        }

        rows.push(ComparisonRow {
            scenario_id: threshold.scenario_id.clone(),
            stage: threshold.stage.clone(),
            base_nanos_per_iter: base_ns,
            head_nanos_per_iter: head_ns,
            delta_pct,
            passed,
        });
    }

    Ok(ComparisonReport { rows, failures })
}

fn parse_stage(value: &str) -> Result<PerfStage, String> {
    match value {
        "env" => Ok(PerfStage::Env),
        "symbol_interning" => Ok(PerfStage::SymbolInterning),
        "arena" => Ok(PerfStage::Arena),
        other => Err(format!("unknown stage: {other}")),
    }
}

fn parse_kind(value: &str) -> Result<PerfScenarioKind, String> {
    match value {
        "parse_only" => Ok(PerfScenarioKind::ParseOnly),
        "eval_fresh" => Ok(PerfScenarioKind::EvalFresh),
        "eval_persistent" => Ok(PerfScenarioKind::EvalPersistent),
        other => Err(format!("unknown scenario kind: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_results_round_trip() {
        let input = concat!(
            "scenario_id\tstage\tkind\tnanos_total\titerations\tnanos_per_iter\n",
            "eval_closure_chain\tenv\teval_fresh\t1000\t10\t100\n"
        );

        let results = parse_results(input).expect("results should parse");
        let result = results
            .get("eval_closure_chain")
            .expect("scenario should be present");

        assert_eq!(result.stage, PerfStage::Env);
        assert_eq!(result.kind, PerfScenarioKind::EvalFresh);
        assert_eq!(result.nanos_total, 1000);
        assert_eq!(result.iterations, 10);
        assert_eq!(result.nanos_per_iter(), 100);
    }

    #[test]
    fn parse_thresholds_round_trip() {
        let input = concat!(
            "scenario_id\tstage\tmin_improve_pct\tmax_regress_pct\trequired\n",
            "eval_closure_chain\tenv\t20\t5\ttrue\n"
        );

        let thresholds = parse_thresholds(input).expect("thresholds should parse");
        assert_eq!(thresholds.len(), 1);
        assert_eq!(thresholds[0].scenario_id, "eval_closure_chain");
        assert_eq!(thresholds[0].min_improve_pct, 20);
        assert_eq!(thresholds[0].max_regress_pct, 5);
        assert!(thresholds[0].required);
    }

    #[test]
    fn compare_results_flags_regression() {
        let mut base = HashMap::new();
        let mut head = HashMap::new();
        base.insert(
            "eval_closure_chain".to_string(),
            PerfResult {
                scenario_id: "eval_closure_chain".to_string(),
                stage: PerfStage::Env,
                kind: PerfScenarioKind::EvalFresh,
                nanos_total: 1000,
                iterations: 10,
            },
        );
        head.insert(
            "eval_closure_chain".to_string(),
            PerfResult {
                scenario_id: "eval_closure_chain".to_string(),
                stage: PerfStage::Env,
                kind: PerfScenarioKind::EvalFresh,
                nanos_total: 1200,
                iterations: 10,
            },
        );
        let thresholds = vec![Threshold {
            scenario_id: "eval_closure_chain".to_string(),
            stage: "env".to_string(),
            min_improve_pct: 20,
            max_regress_pct: 5,
            required: true,
        }];

        let report = compare_results(&base, &head, &thresholds, &["env".to_string()])
            .expect("comparison should work");
        assert_eq!(report.failures, vec!["eval_closure_chain".to_string()]);
        assert!(!report.rows[0].passed);
    }

    #[test]
    fn compare_results_filters_by_stage() {
        let mut base = HashMap::new();
        let mut head = HashMap::new();
        base.insert(
            "parse_symbol_dense".to_string(),
            PerfResult {
                scenario_id: "parse_symbol_dense".to_string(),
                stage: PerfStage::SymbolInterning,
                kind: PerfScenarioKind::ParseOnly,
                nanos_total: 1000,
                iterations: 10,
            },
        );
        head.insert(
            "parse_symbol_dense".to_string(),
            PerfResult {
                scenario_id: "parse_symbol_dense".to_string(),
                stage: PerfStage::SymbolInterning,
                kind: PerfScenarioKind::ParseOnly,
                nanos_total: 1000,
                iterations: 10,
            },
        );
        let thresholds = vec![Threshold {
            scenario_id: "parse_symbol_dense".to_string(),
            stage: "symbol_interning".to_string(),
            min_improve_pct: 25,
            max_regress_pct: 5,
            required: true,
        }];

        let report = compare_results(&base, &head, &thresholds, &["env".to_string()])
            .expect("comparison should work");
        assert!(report.rows.is_empty());
        assert!(report.failures.is_empty());
    }

    #[test]
    fn glob_matches_supports_contains_and_star() {
        assert!(glob_matches("closure", "eval_closure_chain"));
        assert!(glob_matches("eval_*", "eval_closure_chain"));
        assert!(!glob_matches("parse_*", "eval_closure_chain"));
    }
}
