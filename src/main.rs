extern crate cracken;

fn main() {
    if let Err(e) = cracken::runner::run(None) {
        eprintln!("{}", &e);
        std::process::exit(2);
    }
}
