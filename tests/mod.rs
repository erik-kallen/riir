use std::{
    io::{stderr, stdout, Write},
    process::Command,
};

fn run(program: &str, expected_output: &str) {
    let program_path = format!("vendor/tinyvm/programs/tinyvm/{}", program);
    let output = Command::new("cargo")
        .args(&["run", "--example", "tvmi", "--", &program_path])
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

    assert_eq!(result, expected_output);
}

#[test]
fn fact() {
    run(
        "fact.vm",
        "1\n2\n6\n24\n120\n720\n5040\n40320\n362880\n3628800\n",
    );
}
