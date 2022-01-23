use chrono::NaiveDate;
use degiro_tax_report::portfolio::Portfolio;
use degiro_tax_report::Transaction;
use std::fs::File;

fn main() {
    env_logger::init();
    let f = File::open("./testdata/data.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(f);

    let mut transactions = Vec::new();

    for result in rdr.deserialize() {
        let record: Transaction = result.unwrap();
        transactions.push(record);
    }

    let portfolio = Portfolio::with_carry_losses(transactions, 1);

    // 2020 / -2330.4
    // 2021 / 629.5

    let from = NaiveDate::from_ymd(2021, 1, 1);
    let to = NaiveDate::from_ymd(2021, 12, 31);
    let profits = portfolio.report(from, to);

    println!("profits for 2021 {:#?}", profits);
}
