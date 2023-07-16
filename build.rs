use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use std::{env, io};
include!("src/cli.rs");

fn main() -> io::Result<()> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };

    let mut built = Cli::command();

    for &shell in Shell::value_variants() {
        generate_to(shell, &mut built, "xshot", &outdir)?;
    }

    let man = clap_mangen::Man::new(built);
    let mut buffer: Vec<u8> = Vec::new();
    man.render(&mut buffer)?;
    std::fs::write(std::path::PathBuf::from(outdir).join("xshot.1"), buffer)?;

    Ok(())
}
