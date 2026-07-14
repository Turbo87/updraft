fn main() -> anyhow::Result<()> {
    let output_dir = updraft_server::wire::bindings::committed_dir();
    updraft_server::wire::bindings::generate(&output_dir)?;
    println!("Wrote TypeScript bindings to {}", output_dir.display());
    Ok(())
}
