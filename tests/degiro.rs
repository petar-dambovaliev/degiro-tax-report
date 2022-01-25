use chrono::NaiveDate;
use decimal::d128;
use degiro_tax_report::money::Money;
use degiro_tax_report::portfolio::Portfolio;
use degiro_tax_report::Transaction;
use std::fs::File;

#[test]
fn parse_and_calculate() {
    env_logger::init();
    let f = File::open("./testdata/data.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(f);

    let mut transactions = Vec::new();

    for result in rdr.deserialize() {
        let record: Transaction = result.unwrap();
        transactions.push(record);
    }

    let portfolio = Portfolio::with_carry_losses(transactions, 5);

    let from = NaiveDate::from_ymd(2020, 1, 1);
    let to = NaiveDate::from_ymd(2020, 12, 30);
    let report = portfolio.report(from, to);

    assert_eq!(
        report.profit(),
        Money::new(d128!(-2330.423052032860713296898262191196))
    )
}
