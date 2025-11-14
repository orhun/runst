fn main() {
    if let Err(e) = runst::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
