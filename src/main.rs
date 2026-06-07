use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use minipython::RuntimeOptions;

fn usage() -> ! {
    eprintln!("usage: minipython [option] ... [-c cmd | file]");
    process::exit(2);
}

fn run(source: &str, options: RuntimeOptions) -> ! {
    match minipython::run_source_with_runtime_options(source, options) {
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
            run(&source, RuntimeOptions::default());
        }
        eprintln!("minipython: no input");
        process::exit(2);
    }

    let mut options = RuntimeOptions::default();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-b" => {
                options.bytes_warning = (options.bytes_warning + 1).min(2);
                index += 1;
            }
            "-bb" => {
                options.bytes_warning = 2;
                index += 1;
            }
            _ => break,
        }
    }
    if index >= args.len() {
        usage();
    }

    match args[index].as_str() {
        "-c" => {
            if args.len() <= index + 1 {
                usage();
            }
            run(&args[index + 1], options);
        }
        "-e" => {
            if args.len() <= index + 1 {
                usage();
            }
            match minipython::eval_source_with_runtime_options(&args[index + 1], options) {
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
            if args.len() <= index + 1 {
                usage();
            }
            match minipython::compile_source(&args[index + 1]) {
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
            println!("  -b, -bb      warn or error on bytes/string and bytes/int comparisons");
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
            run(&source, options);
        }
    }
}
