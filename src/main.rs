use minipython::run_source;

fn main() {
    let source = "print(1 + 2, 3)";

    match run_source(source) {
        Ok(output) => {
            for line in output {
                println!("{line}");
            }
        }
        Err(message) => eprintln!("{message}"),
    }
}
