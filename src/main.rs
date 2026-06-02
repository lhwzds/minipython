use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

fn usage() -> ! {
    eprintln!("usage: minipython [option] ... [-c cmd | file]");
    process::exit(2);
}

fn run(source: &str) -> ! {
    match minipython::run_source(source) {
        Ok(output) => {
            for line in output {
                println!("{line}");
            }
            process::exit(0);
        }
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        let mut source = String::new();
        if io::stdin().read_to_string(&mut source).is_ok() && !source.is_empty() {
            run(&source);
        }
        eprintln!("minipython: no input");
        process::exit(2);
    }

    match args[1].as_str() {
        "-c" => {
            if args.len() < 3 {
                usage();
            }
            run(&args[2]);
        }
        "-e" => {
            if args.len() < 3 {
                usage();
            }
            match minipython::eval_source(&args[2]) {
                Ok(value) => {
                    println!("{value}");
                    process::exit(0);
                }
                Err(message) => {
                    eprintln!("{message}");
                    process::exit(1);
                }
            }
        }
        "--check" => {
            if args.len() < 3 {
                usage();
            }
            match minipython::compile_source(&args[2]) {
                Ok(()) => process::exit(0),
                Err(message) => {
                    eprintln!("{message}");
                    process::exit(1);
                }
            }
        }
        "-h" | "--help" => {
            println!("minipython [option] ... [-c cmd | file]");
            println!();
            println!("options:");
            println!("  -c cmd       execute program passed as string");
            println!("  -e expr      evaluate expression and print result");
            println!("  --check src  compile and check for errors");
            println!("  -h, --help   show this help");
            process::exit(0);
        }
        arg if arg.starts_with('-') => {
            eprintln!("minipython: unknown option: {arg}");
            process::exit(2);
        }
        file => {
            let source = fs::read_to_string(file).unwrap_or_else(|e| {
                eprintln!("minipython: {file}: {e}");
                process::exit(2);
            });
            run(&source);
        }
    }
}
