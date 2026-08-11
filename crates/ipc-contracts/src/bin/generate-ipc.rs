use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: generate-ipc <output-file>")?;
    if env::args_os().nth(2).is_some() {
        return Err("usage: generate-ipc <output-file>".into());
    }

    let parent = output
        .parent()
        .ok_or("output file must have a parent directory")?;
    fs::create_dir_all(parent)?;
    fs::write(output, ipc_contracts::typescript_declarations())?;
    Ok(())
}
