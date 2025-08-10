use std::process::Command;

#[test]
fn sim_binary_smoke() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "sim", "--", "1", "2"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run sim binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("non utf8 output");
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("invalid json");
    assert!(v["winner"].is_string());
}
