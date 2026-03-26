use criterion::{Criterion, black_box, criterion_group, criterion_main};
use lisp::perf_support::{all_scenarios, run_scenario_once};

fn bench_perf_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("lisp_perf");

    for scenario in all_scenarios() {
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
