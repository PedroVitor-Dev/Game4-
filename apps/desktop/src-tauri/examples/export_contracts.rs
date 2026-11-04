use game4_desktop_lib::contracts::export;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = export::target_directory();
    let artifacts = [
        ("src/generated.ts", export::typescript()),
        ("examples.json", export::examples()),
    ];
    let check = std::env::args().skip(1).any(|arg| arg == "--check");
    for (relative, expected) in artifacts {
        let path = directory.join(relative);
        if check {
            if std::fs::read_to_string(&path)? != expected {
                return Err(
                    format!("Stale contract artifact: {relative}; run contracts:generate").into(),
                );
            }
        } else {
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, expected)?;
        }
    }
    println!(
        "IPC contract artifacts {}",
        if check { "verified" } else { "generated" }
    );
    Ok(())
}
