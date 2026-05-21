use minipython::run_source;

fn main() {
    let source = "x = 1 + 2\nprint(x == 3)";

    match run_source(source) {
        Ok(output) => {
            for line in output {
                println!("{line}");
            }
        }
        Err(message) => eprintln!("{message}"),
    }
}
