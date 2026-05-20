use minipython::run_source;

fn main() {
    let source = "name = \"minipython\"\nprint(\"hello\", name)";

    match run_source(source) {
        Ok(output) => {
            for line in output {
                println!("{line}");
            }
        }
        Err(message) => eprintln!("{message}"),
    }
}
