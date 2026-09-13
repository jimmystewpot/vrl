use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use vrl::compiler::state::RuntimeState;
use vrl::compiler::{Context, TimeZone};
use vrl::value::Value;

const SIZES: [usize; 3] = [100, 1_000, 10_000];

const BREAK_SRC: &str = r#"
found = null
for_each(.) -> |_index, item| {
    if item == 5 {
        found = item
        break
    }
}
found
"#;

const FLAG_SRC: &str = r#"
found = null
for_each(.) -> |_index, item| {
    if found == null {
        if item == 5 {
            found = item
        }
    }
}
found
"#;

fn benchmark_break_vs_flag(c: &mut Criterion) {
    let fns = vrl::stdlib::all();
    let break_prog = vrl::compiler::compile(BREAK_SRC, &fns)
        .expect("failed to compile break VRL program")
        .program;
    let flag_prog = vrl::compiler::compile(FLAG_SRC, &fns)
        .expect("failed to compile flag VRL program")
        .program;

    let mut group = c.benchmark_group("for_each_early_exit");

    for &size in &SIZES {
        let array: Value = (0..size)
            .map(|i| Value::Integer(i as i64))
            .collect::<Vec<_>>()
            .into();

        group.bench_with_input(BenchmarkId::new("break", size), &array, |b, arr| {
            b.iter_batched(
                || {
                    let state = RuntimeState::default();
                    let target = arr.clone();
                    (state, target)
                },
                |(mut state, mut target)| {
                    let tz = TimeZone::default();
                    let mut ctx = Context::new(&mut target, &mut state, &tz);
                    let res = break_prog.resolve(&mut ctx);
                    debug_assert_eq!(res, Ok(Value::Integer(5)));
                    res
                },
                criterion::BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("flag", size), &array, |b, arr| {
            b.iter_batched(
                || {
                    let state = RuntimeState::default();
                    let target = arr.clone();
                    (state, target)
                },
                |(mut state, mut target)| {
                    let tz = TimeZone::default();
                    let mut ctx = Context::new(&mut target, &mut state, &tz);
                    let res = flag_prog.resolve(&mut ctx);
                    debug_assert_eq!(res, Ok(Value::Integer(5)));
                    res
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_break_vs_flag);
criterion_main!(benches);
