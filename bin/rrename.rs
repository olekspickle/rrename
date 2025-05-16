use clap::Parser;
use rrename::Rrename;

fn main() -> anyhow::Result<()> {
    if std::env::args().len() == 1 {
        // only binary name was provided
        println!("rrename does nothing by default.\n\nTo see all options run: rrename --help");
        return Ok(());
    }

    let cli = Rrename::parse();
    cli.run()?;

    Ok(())
}
