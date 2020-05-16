use std::{env, fs, process::exit};
use tinyvm::context::Program;

fn main() {
    let mut args = env::args();
    if args.len() != 2 {
        println!("Usage: `tvmi file`");
        exit(1);
    }

    let filename = args.nth(1).unwrap();

    let source = match read_to_string_with_possible_extension(&filename, ".vm") {
        Ok(s) => s,
        Err(_) => {
            println!("Error reading file {}", filename);
            exit(1);
        }
    };

    let program = match Program::load(source) {
        Ok(p) => p,
        Err(e) => {
            println!("Error {:?}", e);
            exit(1);
        }
    };

    match program.run() {
        Ok(_) => {},
        Err(e) => {
            println!("Error executing program: {:?}", e);
            exit(1);
        }
    }
}

fn read_to_string_with_possible_extension(
    filename: &str,
    extension: &str,
) -> Result<String, std::io::Error> {
    match fs::read_to_string(filename) {
        Ok(s) => return Ok(s),
        Err(error) => match error.kind() {
            std::io::ErrorKind::NotFound => (),
            _ => return Err(error),
        },
    };

    fs::read_to_string(filename.to_owned() + extension)
}
