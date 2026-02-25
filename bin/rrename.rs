use clap::Parser;
use rrename::RrenameCli;

fn main() -> anyhow::Result<()> {
    if std::env::args().len() == 1 {
        // only binary name was provided
        println!(
            "rrename does nothing by default because of its destructive nature.\n\nTo see all options run: rrename --help"
        );
        return Ok(());
    }

    let cli = RrenameCli::parse();
    tracing_subscriber::fmt()
        .with_max_level(cli.verbosity)
        .init();
    cli.run()?;

    Ok(())
}
