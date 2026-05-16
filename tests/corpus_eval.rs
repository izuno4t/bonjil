use std::fs;
use std::process::Command;

#[test]
fn corpus_eval_outputs_comparison_report() {
    let root = "target/corpus-eval-test/input";
    fs::create_dir_all(root).unwrap();
    fs::write(format!("{root}/sample.html"), "<h1>Title</h1><p>Body</p>").unwrap();

    let bin = std::env::var("CARGO_BIN_EXE_bonjil-corpus-eval")
        .expect("bonjil-corpus-eval binary path is missing");
    let output = Command::new(bin)
        .arg("--root")
        .arg(root)
        .arg("--out")
        .arg("target/corpus-eval-test/report.json")
        .arg("--limit")
        .arg("1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fs::read_to_string("target/corpus-eval-test/report.json").unwrap();
    assert!(report.contains("\"tool\":\"bonjil\""));
    assert!(report.contains("\"summary\""));
    assert!(report.contains("\"superiority_claim\""));
}

#[test]
fn corpus_eval_can_filter_by_extension() {
    let root = "target/corpus-eval-filter-test/input";
    fs::create_dir_all(root).unwrap();
    fs::write(format!("{root}/sample.html"), "<h1>Title</h1><p>Body</p>").unwrap();
    fs::write(format!("{root}/sample.txt"), "plain text").unwrap();

    let bin = std::env::var("CARGO_BIN_EXE_bonjil-corpus-eval")
        .expect("bonjil-corpus-eval binary path is missing");
    let output = Command::new(bin)
        .arg("--root")
        .arg(root)
        .arg("--out")
        .arg("target/corpus-eval-filter-test/report.json")
        .arg("--limit")
        .arg("10")
        .arg("--per-ext")
        .arg("10")
        .arg("--ext")
        .arg("txt")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fs::read_to_string("target/corpus-eval-filter-test/report.json").unwrap();
    assert!(report.contains("\"txt\":1"));
    assert!(!report.contains("\"html\":1"));
}
