use decimal::d128;
use lazy_static::lazy_static;
use regex::Regex;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[non_exhaustive]
pub enum Error {
    Currency(Option<String>, Option<String>),
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Currency(left, right) => {
                f.write_str(&format!("left: {:#?} right: {:#?}", left, right))
            }
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize, Clone, Default, PartialEq)]
pub struct Money {
    pub(crate) amount: d128,
    currency: Option<String>,
}

impl Money {
    pub fn new(amount: d128) -> Self {
        Self {
            amount,
            currency: None,
        }
    }

    pub fn with_currency(amount: d128, currency: String) -> Self {
        Self {
            amount,
            currency: Some(currency.to_lowercase()),
        }
    }

    pub fn add(&mut self, rhs: &Self) -> Result<(), Error> {
        self.validate(&rhs)?;
        self.amount += rhs.amount;
        Ok(())
    }

    pub fn sub(&mut self, rhs: &Self) -> Result<(), Error> {
        self.validate(&rhs)?;
        self.amount -= rhs.amount;
        Ok(())
    }

    pub fn div(&mut self, rhs: isize) -> Result<(), Error> {
        self.amount /= d128::from(rhs as i64);
        Ok(())
    }

    pub fn mul(&mut self, rhs: isize) -> Result<(), Error> {
        self.amount *= d128::from(rhs as i64);
        Ok(())
    }

    pub fn truncate_trailing_zeros(&self) -> Self {
        Self {
            amount: self.amount.reduce(),
            currency: self.currency.clone(),
        }
    }

    pub fn abs(&self) -> Self {
        Self {
            amount: self.amount.abs(),
            currency: self.currency.clone(),
        }
    }

    pub fn is_negative(&self) -> bool {
        self.amount.is_negative()
    }

    fn validate(&self, rhs: &Self) -> Result<(), Error> {
        let currency = rhs.currency.as_ref().map(|c| c.to_lowercase());
        if self.currency != currency {
            return Err(Error::Currency(self.currency.clone(), currency));
        }
        Ok(())
    }
}

impl FromStr for Money {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE1: Regex = Regex::new(r"^(?i)([a-z]+)\s*([-]?[0-9,.]+)\s*$").unwrap();
            static ref RE2: Regex = Regex::new(r"^(?i)([-]?[0-9,.]+)\s*([a-z]+)\s*$").unwrap();
        }

        for cap in RE1.captures_iter(s) {
            let currency = cap[0].to_lowercase();
            let amount =
                d128::from_str(&cap[1]).map_err(|_| format!("invalid amount: `{}`", &cap[1]))?;

            return Ok(Self {
                amount,
                currency: Some(currency),
            });
        }

        for cap in RE2.captures_iter(s) {
            let currency = cap[1].to_lowercase();
            let amount =
                d128::from_str(&cap[0]).map_err(|_| format!("invalid amount: `{}`", &cap[1]))?;

            return Ok(Self {
                amount,
                currency: Some(currency),
            });
        }

        s.parse::<d128>()
            .map(|d| Self {
                amount: d,
                currency: None,
            })
            .map_err(|_| format!("invalid input: `{}`", s))
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Money, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(MoneyVisitor)
    }
}

struct MoneyVisitor;

impl<'de> Visitor<'de> for MoneyVisitor {
    type Value = Money;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_str<E>(self, value: &str) -> Result<Money, E>
    where
        E: de::Error,
    {
        Money::from_str(value).map_err(|e| de::Error::custom(e))
    }
}
