use std::{
    io::{stderr, stdout, Write},
    process::Command,
};

fn run(program: &str, expected_output: &[i32]) {
    let output = Command::new("cargo")
        .args(&["run", "--example", "tvmi", "--", program])
        .output()
        .expect(&format!("Failed to execute {}", program));

    if !output.status.success() {
        stdout().write(&output.stdout).unwrap();
        stderr().write(&output.stderr).unwrap();
        panic!(
            "Execution of {} resulted in status {}",
            program,
            output.status.code().unwrap()
        );
    }

    let result = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n");

    let result: &str = &result;

    let actual_output: Vec<i32> = result
        .split("\n")
        .filter(|s| s.len() > 0)
        .map(|s| s.parse::<i32>().unwrap())
        .collect();

    assert_eq!(actual_output, expected_output);
}

fn run_vendor(program: &str, expected_output: &[i32]) {
    run(
        &format!("vendor/tinyvm/programs/tinyvm/{}", program),
        expected_output,
    );
}

fn run_local(program: &str, expected_output: &[i32]) {
    run(&format!("tests/{}", program), expected_output);
}

#[test]
fn fact() {
    run_vendor(
        "fact.vm",
        &[1, 2, 6, 24, 120, 720, 5040, 40320, 362880, 3628800],
    );
}

#[test]
fn instructions() {
    run_local(
        "instructions.vm",
        &[
            1, 2, 1, 0, 2, 4, 2, 7, 1, 12, 3, 2, -5, 59, 4, 63, 20, 6, 2, 10, 11, 100, 102, 200,
            202, 300, 301, 303, 401, 403, 500, 502, 503, 602, 603,
        ],
    );
}

#[test]
fn operands() {
    run_local(
        "operands.vm",
        &[
            2, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 200,
            201, 300, 301, 302, 303, 304, 305, 306, -300, -301, -302, -303, -304, -305, -306, 3,
        ],
    );
}
