pub mod money;
pub mod portfolio;

use anyhow::anyhow;
use chrono::{NaiveDate, NaiveTime};
use csv::DeserializeRecordsIter;
use dateparser::parse;
use futures::Stream;
use money::Money;
use rev_lines::RevLines;
use serde::{de, Deserialize, Serialize};
use std::fmt::Write;
use std::fs::File;
use std::io::BufReader;
use std::iter::Peekable;
use std::pin::Pin;
use std::task::{Context, Poll};

pub enum TransactionType {
    Buy,
    Sell,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Transaction {
    #[serde(deserialize_with = "deserialize_naive_date")]
    date: NaiveDate,
    #[serde(deserialize_with = "deserialize_naive_time")]
    time: NaiveTime,
    product: String,
    #[serde(rename(deserialize = "ISIN"))]
    isin: String,
    reference: String,
    quantity: isize,
    venue: String,
    price: Money,
    #[serde(rename(deserialize = "Local value"))]
    local_value: Money,
    value: Money,
    #[serde(rename(deserialize = "Transaction and/or third"))]
    transaction: Option<String>,
    #[serde(rename(deserialize = "Exchange rate"))]
    exchange_rate: Option<String>,
    total: String,
    #[serde(rename(deserialize = "Order ID"))]
    order_id: String,
}

#[derive(Debug)]
pub enum TransactionError {
    SellWithNegPrice { order_id: String },
    BuyingWithNegPrice { order_id: String },
}

impl Transaction {
    pub fn new(
        date: NaiveDate,
        isin: String,
        quantity: isize,
        value: Money,
        order_id: String,
    ) -> Result<Self, TransactionError> {
        if quantity.is_negative() && value.amount.is_negative() {
            return Err(TransactionError::SellWithNegPrice { order_id });
        }

        if quantity.is_positive() && value.amount.is_positive() {
            return Err(TransactionError::BuyingWithNegPrice { order_id });
        }

        Ok(Self {
            date,
            time: NaiveTime::from_hms(1, 1, 1),
            product: "".to_string(),
            isin,
            reference: "".to_string(),
            quantity,
            venue: "".to_string(),
            price: Default::default(),
            value,
            local_value: Default::default(),
            transaction: None,
            exchange_rate: None,
            total: "".to_string(),
            order_id,
        })
    }

    pub fn new_unchecked(
        date: NaiveDate,
        isin: String,
        quantity: isize,
        value: Money,
        order_id: String,
    ) -> Self {
        match Self::new(date, isin, quantity, value, order_id) {
            Ok(ok) => ok,
            Err(e) => panic!("{:#?}", e),
        }
    }

    pub fn date(&self) -> &NaiveDate {
        &self.date
    }

    pub fn r#type(&self) -> TransactionType {
        match self.value.amount.is_negative() {
            true => TransactionType::Buy,
            false => TransactionType::Sell,
        }
    }
}

fn deserialize_naive_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = de::Deserialize::deserialize(deserializer)?;
    let dt = match local_date_parse(s) {
        Ok(dt) => dt,
        Err(parse_err) => {
            let dt = parse(s)
                .map_err(|e| de::Error::custom(format!("error: {} error: {}", parse_err, e)))?;
            dt.naive_local().date()
        }
    };
    Ok(dt)
}

fn deserialize_naive_time<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = de::Deserialize::deserialize(deserializer)?;
    let nt = match local_time_parse(s) {
        Ok(dt) => dt,
        Err(parse_err) => {
            let dt = parse(s).map_err(|e| de::Error::custom(format!("{}\n{}", parse_err, e)))?;
            dt.naive_local().time()
        }
    };
    Ok(nt)
}

fn local_date_parse(s: &str) -> Result<NaiveDate, chrono::ParseError> {
    NaiveDate::parse_from_str(s, "%d-%m-%Y")
}

fn local_time_parse(s: &str) -> Result<NaiveTime, chrono::ParseError> {
    NaiveTime::parse_from_str(s, "%H:%M")
}

pub struct CsvStream {
    parser: ReverseCsv,
}

impl CsvStream {
    pub fn new(file: File) -> std::io::Result<Self> {
        let parser = ReverseCsv::new(file)?;
        Ok(Self { parser })
    }
}

impl Stream for CsvStream {
    type Item = anyhow::Result<Transaction>;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = self
            .as_mut()
            .parser
            .next()
            .map(|res| res.map_err(|e| anyhow!("{}", e)));
        Poll::Ready(next)
    }
}

struct ReverseCsv {
    rev_lines: Peekable<RevLines<File>>,
}

impl ReverseCsv {
    pub fn new(file: File) -> std::io::Result<Self> {
        let reader = BufReader::new(file);
        let rev_lines = RevLines::new(reader)?.peekable();

        Ok(Self { rev_lines })
    }
}

const HEADERS: &str = "Date,Time,Product,ISIN,Reference,Venue,Quantity,Price,,Local value,,Value,,Exchange rate,Transaction and/or third,,Total,,Order ID\n";

impl Iterator for ReverseCsv {
    type Item = anyhow::Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.rev_lines.next()?;
        // if it is the first/headers skip
        self.rev_lines.peek()?;

        let mut input = HEADERS.to_string();
        input.write_str(&line).unwrap();

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(input.as_bytes());
        let mut iter: DeserializeRecordsIter<_, Transaction> = rdr.deserialize();

        let item = iter.next()?;

        assert!(iter.peekable().peek().is_none());

        let res = match item {
            Ok(k) => anyhow::Result::Ok(k),
            Err(e) => anyhow::Result::Err(anyhow!("{}", e)),
        };

        Some(res)
    }
}
