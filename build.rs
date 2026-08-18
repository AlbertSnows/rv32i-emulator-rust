use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let fixtures_dir = "tests/fixtures";
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_riscv_tests.rs");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut generated = String::new();
    for entry in fs::read_dir(fixtures_dir).unwrap() {
        let file_name = entry.unwrap().file_name().into_string().unwrap();
        let fixture_path = format!("{}/tests/fixtures/{}", manifest_dir, file_name);
        // fixture name looks like "add-p-add", everything before -p- is the instruction name
        let test_name = file_name.split("-p-").next().unwrap();
        generated.push_str(&format!(
            "riscv_test!(test_rv32ui_p_{}_passes, {:?});\n",
            test_name,
            fixture_path
        ));

    }
    fs::write(&dest_path, generated).unwrap();
    // tells Cargo to re-run this script (and regenerate the list) whenever
    // the fixtures folder changes 
    // without this, adding a new fixture
    // file might not actually trigger regeneration on the next build.
    println!("cargo:rerun-if-changed={}", fixtures_dir);
}