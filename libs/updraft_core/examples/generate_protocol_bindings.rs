fn main() -> std::io::Result<()> {
    let output_dir = updraft_core::bindings::committed_dir();
    updraft_core::bindings::generate(&output_dir)?;
    println!("wrote TypeScript bindings to {}", output_dir.display());
    Ok(())
}
