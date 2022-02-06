use clap::{ArgEnum, Parser, Subcommand};
use degiro_tax_report::portfolio::Portfolio;
use degiro_tax_report::CsvStream;
use std::fs::File;
use std::io::Write;

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    #[clap(short, long)]
    file: String,
    #[clap(short, long)]
    year: i32,
    #[clap(short, long, default_value_t = 0)]
    carry_losses_years: u8,

    #[clap(subcommand)]
    args: Args,
}

/// Simple program to greet a person
#[derive(Debug, ArgEnum, Clone, Subcommand)]
enum Args {
    Adjusted,
    Unadjusted,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = Cli::parse();
    let f = File::open(cli.file).unwrap();
    let tr_stream = CsvStream::new(f).unwrap();
    let portfolio = Portfolio::with_carry_losses(tr_stream, cli.carry_losses_years);

    let profits = portfolio.report(cli.year).await.unwrap();

    let report = match cli.args {
        Args::Adjusted => profits.adjusted_profit().unwrap(),
        Args::Unadjusted => profits.profit().unwrap(),
    };

    std::io::stdout()
        .write(format!("report: {}", report).as_bytes())
        .unwrap();
}
