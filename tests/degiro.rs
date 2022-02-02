use csv::DeserializeRecordsIter;
use decimal::d128;
use degiro_tax_report::money::Money;
use degiro_tax_report::portfolio::Portfolio;
use degiro_tax_report::{CsvStream, Transaction};
use std::fs::File;

#[tokio::test]
async fn parse_and_calculate() {
    env_logger::init();
    let f = File::open("./testdata/data.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(f);
    let iter: DeserializeRecordsIter<File, Transaction> = rdr.deserialize();
    let tr_stream = CsvStream::new(iter);

    let portfolio = Portfolio::with_carry_losses(tr_stream, 5);

    let report = portfolio.report(2021).await.unwrap();

    assert_eq!(report.adjusted_profit().unwrap(), Money::new(d128!(0)))
}
