//! M7-C performance gate for the styling expression engine.
//!
//! Gate: evaluating a representative styling expression across **10000
//! features** (one "frame" of styling work) must stay **<= 1 ms**. Each
//! criterion iteration below evaluates the expression once per feature over a
//! 10000-feature batch, so the reported per-iteration time *is* the per-frame
//! styling cost. Run with:
//!
//! ```text
//! cargo bench -p cesium-styling --offline
//! ```
//!
//! This file replaces the temporary placeholder stub that another agent added
//! to unblock workspace resolution; it is the real M7-C deliverable.

use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use cesium_styling::{Expression, ExpressionFeature, Value};

/// Minimal feature backed by a property map (mirrors the test MockFeature).
struct BenchFeature {
    props: HashMap<String, Value>,
}

impl BenchFeature {
    fn new(i: f64) -> Self {
        let mut props = HashMap::new();
        props.insert("red".to_string(), Value::Number(i % 255.0));
        props.insert("green".to_string(), Value::Number((i * 2.0) % 255.0));
        props.insert("blue".to_string(), Value::Number((i * 3.0) % 255.0));
        props.insert("a".to_string(), Value::Number(0.5 + (i % 10.0) * 0.05));
        props.insert("height".to_string(), Value::Number(i));
        BenchFeature { props }
    }
}

impl ExpressionFeature for BenchFeature {
    fn get_property_inherited(&self, name: &str) -> Option<Value> {
        self.props.get(name).cloned()
    }
}

const FEATURE_COUNT: usize = 10_000;

fn make_features() -> Vec<BenchFeature> {
    (0..FEATURE_COUNT).map(|i| BenchFeature::new(i as f64)).collect()
}

/// The headline gate: one styling pass over 10000 features (<= 1 ms/frame).
/// Uses a color expression with 4 variable substitutions plus arithmetic — a
/// representative per-feature styling evaluation.
fn bench_eval_color_10000_features(c: &mut Criterion) {
    let expr = Expression::try_new("rgba(${red}, ${green}, ${blue}, ${a} * 0.5)", None)
        .expect("expression parses");
    let features = make_features();

    c.bench_function("styling_eval/color_10000_features", |b| {
        b.iter(|| {
            let mut acc = 0.0;
            for f in &features {
                if let Ok(Value::Cartesian4(col)) = expr.evaluate(Some(f)) {
                    acc += col.x + col.w;
                }
            }
            black_box(acc)
        })
    });
}

/// A conditional + comparison + math-function expression over 10000 features,
/// exercising the branch/ternary/builtin-function evaluation paths.
fn bench_eval_conditional_10000_features(c: &mut Criterion) {
    let expr = Expression::try_new(
        "(${height} > 50.0) ? clamp(${height} / 100.0, 0.0, 1.0) : mix(0.0, 1.0, ${a})",
        None,
    )
    .expect("expression parses");
    let features = make_features();

    c.bench_function("styling_eval/conditional_10000_features", |b| {
        b.iter(|| {
            let mut acc = 0.0;
            for f in &features {
                if let Ok(Value::Number(n)) = expr.evaluate(Some(f)) {
                    acc += n;
                }
            }
            black_box(acc)
        })
    });
}

/// String-template interpolation over 10000 features — exercises the
/// `VariableInString` path. Before the `OnceLock` fix this recompiled the
/// `${...}` regex on *every* evaluation; string templates are the most common
/// real-world styling expression, so this bench guards that hot path.
fn bench_eval_string_template_10000_features(c: &mut Criterion) {
    let expr =
        Expression::try_new("'h:${height}/a:${a}'", None).expect("expression parses");
    let features = make_features();

    c.bench_function("styling_eval/string_template_10000_features", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for f in &features {
                if let Ok(Value::String(s)) = expr.evaluate(Some(f)) {
                    acc += s.len();
                }
            }
            black_box(acc)
        })
    });
}

/// Single-expression evaluation cost (per feature), for reference.
fn bench_single_eval(c: &mut Criterion) {
    let expr = Expression::try_new("rgba(${red}, ${green}, ${blue}, ${a} * 0.5)", None)
        .expect("expression parses");
    let feature = BenchFeature::new(42.0);

    c.bench_function("styling_eval/single_eval", |b| {
        b.iter(|| black_box(expr.evaluate(Some(&feature)).is_ok()))
    });
}

/// Parse (tokenize + build runtime AST) cost, for reference — parse happens
/// once per style, evaluate happens per feature per frame.
fn bench_parse(c: &mut Criterion) {
    c.bench_function("styling_eval/parse", |b| {
        b.iter(|| {
            black_box(
                Expression::try_new(
                    "(${height} > 50.0) ? rgba(${red}, ${green}, ${blue}, ${a}) : color('blue')",
                    None,
                )
                .is_ok(),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_eval_color_10000_features,
    bench_eval_conditional_10000_features,
    bench_eval_string_template_10000_features,
    bench_single_eval,
    bench_parse,
);
criterion_main!(benches);
