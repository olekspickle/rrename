use clap::Parser;
use rrename::Rrename;

fn main() -> anyhow::Result<()> {
    let cli = Rrename::parse();
    cli.run()?;

    Ok(())
}
