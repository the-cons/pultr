fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => println!("pultr {}", env!("CARGO_PKG_VERSION")),
        _ => {
            println!(
                "pultr {} - pre-alpha, nothing to see yet.",
                env!("CARGO_PKG_VERSION")
            );
            println!("Follow along: https://github.com/the-cons/pultr")
        }
    }
}
