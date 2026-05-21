use minipython::run_source;

fn main() {
    let source = "x = 1 + 2\nif x == 3:\n    print(\"yes\")";

    match run_source(source) {
        Ok(output) => {
            for line in output {
                println!("{line}");
            }
        }
        Err(message) => eprintln!("{message}"),
    }
}
