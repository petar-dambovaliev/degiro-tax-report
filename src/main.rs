use csv::DeserializeRecordsIter;
use degiro_tax_report::portfolio::Portfolio;
use degiro_tax_report::{CsvStream, Transaction};
use std::fs::File;

#[tokio::main]
async fn main() {
    env_logger::init();
    let f = File::open("./testdata/data.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(f);
    let iter: DeserializeRecordsIter<File, Transaction> = rdr.deserialize();
    let tr_stream = CsvStream::new(iter);

    let portfolio = Portfolio::with_carry_losses(tr_stream, 1);

    // 2020 / -2330.4
    // 2021 / 629.5

    let profits = portfolio.report(2021).await.unwrap();

    println!("profits for 2021 {:#?}", profits.profit());
    println!(
        "profits for 2021, adjusted with carry over losses {:#?}",
        profits.adjusted_profit()
    );
}
