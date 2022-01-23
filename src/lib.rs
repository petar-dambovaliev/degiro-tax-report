pub mod money;
pub mod portfolio;

use chrono::{NaiveDate, NaiveTime};
use dateparser::parse;
use money::Money;
use serde::{de, Deserialize, Serialize};

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

pub enum TransactionError {
    SellWithNegPrice { order_id: String },
    BuyingWithNegPrice { order_id: String },
}

impl Transaction {
    pub fn new(
        date: NaiveDate,
        isin: String,
        quantity: isize,
        local_value: Money,
        order_id: String,
    ) -> Result<Self, TransactionError> {
        if quantity.is_negative() && local_value.amount.is_negative() {
            return Err(TransactionError::SellWithNegPrice { order_id });
        }

        if quantity.is_positive() && local_value.amount.is_positive() {
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
            local_value,
            value: Default::default(),
            transaction: None,
            exchange_rate: None,
            total: "".to_string(),
            order_id,
        })
    }

    pub fn date(&self) -> &NaiveDate {
        &self.date
    }

    pub fn r#type(&self) -> TransactionType {
        match self.local_value.amount.is_negative() {
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
