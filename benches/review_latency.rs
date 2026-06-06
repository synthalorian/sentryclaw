use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sentryshark::diff_filter::DiffFilter;
use sentryshark::inline_comments::ReviewParser;
use sentryshark::llm::LlmClient;
use sentryshark::config::ReviewConfig;
use sentryshark::rule_engine::{RuleEngine, ReviewRule, SeverityLevel};
use sentryshark::auto_approve::{AutoApprover, AutoApproveConfig};

fn benchmark_diff_filter(c: &mut Criterion) {
    let diff = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    let x = 1;
+    let x = 2;
 }
 diff --git a/Cargo.lock b/Cargo.lock
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,5 +1,5 @@
 version = 1
-diff = old
+diff = new
"#;

    let filter = DiffFilter::new(
        &["Cargo.lock".to_string(), "*.lock".to_string()],
        &["dist/".to_string()],
        true,
    );

    c.bench_function("diff_filter", |b| {
        b.iter(|| filter.filter_diff(black_box(diff)))
    });
}

fn benchmark_review_parser(c: &mut Criterion) {
    let llm_output = r#"VERDICT: COMMENT
SUMMARY: Some issues found in the code review

FILE: src/main.rs
LINE: 42
COMMENT: This could panic, consider using unwrap_or_default instead

FILE: src/lib.rs
LINE: 10
COMMENT: Good documentation here
"#;

    c.bench_function("review_parser", |b| {
        b.iter(|| ReviewParser::parse(black_box(llm_output)))
    });
}

fn benchmark_prompt_building(c: &mut Criterion) {
    let config = ReviewConfig {
        security: true,
        style: true,
        performance: true,
        correctness: true,
        maintainability: true,
        inline_comments: true,
        summary_comment: true,
        template: None,
    };

    let client = LlmClient::new(
        "http://localhost:8080".to_string(),
        "test-model".to_string(),
        4096,
        0.1,
        config,
    );

    let diff = "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,5 +1,5 @@\n fn main() {\n-    let x = 1;\n+    let x = 2;\n }\n";

    c.bench_function("prompt_building", |b| {
        b.iter(|| client.build_prompt(black_box(diff)))
    });
}

fn benchmark_rule_engine(c: &mut Criterion) {
    let mut engine = RuleEngine::new();
    engine.add_rules(vec![
        ReviewRule {
            name: "no_unwrap".to_string(),
            description: "Avoid unwrap".to_string(),
            pattern: r"unwrap\(\)".to_string(),
            severity: SeverityLevel::Warning,
            message: "Consider unwrap_or".to_string(),
        },
        ReviewRule {
            name: "no_panic".to_string(),
            description: "Avoid panic".to_string(),
            pattern: r"panic!".to_string(),
            severity: SeverityLevel::Critical,
            message: "Don't panic".to_string(),
        },
    ]);

    let diff = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    let x = Some(1);
+    let x = Some(1).unwrap();
+    panic!("oh no");
 }
"#;

    c.bench_function("rule_engine_check", |b| {
        b.iter(|| engine.check_diff(black_box(diff)))
    });
}

fn benchmark_auto_approve(c: &mut Criterion) {
    let config = AutoApproveConfig {
        enabled: true,
        docs_patterns: vec!["*.md".to_string(), "README".to_string()],
        skip_lockfiles: true,
        skip_whitespace: true,
    };

    let lockfile_diff = r#"diff --git a/Cargo.lock b/Cargo.lock
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,5 +1,5 @@
 version = 1
-diff = old
+diff = new
"#;

    c.bench_function("auto_approve_lockfile", |b| {
        b.iter(|| AutoApprover::is_trivial(black_box(lockfile_diff), &config))
    });
}

fn benchmark_large_diff_filtering(c: &mut Criterion) {
    // Generate a large diff with many files
    let mut large_diff = String::new();
    for i in 0..100 {
        large_diff.push_str(&format!(
            "diff --git a/src/file{}.rs b/src/file{}.rs\n--- a/src/file{}.rs\n+++ b/src/file{}.rs\n@@ -1,5 +1,5 @@\n fn func{}() {{\n-    let x = 1;\n+    let x = 2;\n }}\n",
            i, i, i, i, i
        ));
    }
    // Add some lockfiles that should be filtered
    for i in 0..10 {
        large_diff.push_str(&format!(
            "diff --git a/Cargo{}.lock b/Cargo{}.lock\n--- a/Cargo{}.lock\n+++ b/Cargo{}.lock\n@@ -1 +1 @@\n-old\n+new\n",
            i, i, i, i
        ));
    }

    let filter = DiffFilter::new(
        &["*.lock".to_string()],
        &["dist/".to_string()],
        true,
    );

    c.bench_function("large_diff_filter", |b| {
        b.iter(|| filter.filter_diff(black_box(&large_diff)))
    });
}

criterion_group!(
    benches,
    benchmark_diff_filter,
    benchmark_review_parser,
    benchmark_prompt_building,
    benchmark_rule_engine,
    benchmark_auto_approve,
    benchmark_large_diff_filtering
);
criterion_main!(benches);
