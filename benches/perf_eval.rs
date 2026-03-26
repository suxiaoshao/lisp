use criterion::{Criterion, black_box, criterion_group, criterion_main};
use lisp::perf_support::{PerfStage, filtered_scenarios, run_scenario_once};

fn bench_perf_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("lisp_perf");
    let stage = std::env::var("LISP_PERF_STAGE")
        .ok()
        .map(|value| value.parse::<PerfStage>().expect("valid perf stage"));
    let filter = std::env::var("LISP_PERF_FILTER").ok();

    for scenario in filtered_scenarios(stage, filter.as_deref()) {
        group.bench_function(scenario.id, |b| {
            b.iter(|| {
                run_scenario_once(black_box(scenario)).expect("benchmark scenario should run")
            })
        });
    }

    group.finish();
}

criterion_group!(perf_benches, bench_perf_scenarios);
criterion_main!(perf_benches);
