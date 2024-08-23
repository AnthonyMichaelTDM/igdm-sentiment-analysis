mod analyzer;
mod parser;
mod plotter;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use parser::ConversationDirectory;

#[derive(Parser, Debug)]
#[command(version, author, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "input directory containing message json files")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let analysis = ConversationDirectory::try_from(args.path)
        .unwrap()
        .parse()?
        .analyze();

    // if it doesn't already exist, create the output directory
    let output_dir = "./output";
    if !PathBuf::from(output_dir).exists() {
        std::fs::create_dir(output_dir)?;
    }

    // generate every plot
    for plot_type in [
        plotter::PlotType::Positive,
        plotter::PlotType::Negative,
        plotter::PlotType::Neutral,
        plotter::PlotType::Compound,
    ] {
        analysis.plot(
            plot_type,
            &PathBuf::from(format!("{output_dir}/{plot_type}.png")),
        )?;
    }

    Ok(())
}
