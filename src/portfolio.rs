use crate::{Money, Transaction, TransactionType};
use chrono::{Datelike, NaiveDate};
use std::collections::{HashMap, VecDeque};
use std::ops::Add;

pub struct Portfolio {
    transactions: VecDeque<Transaction>,
    years_carry_losses: u8,
}

impl Portfolio {
    pub fn new(mut transactions: Vec<Transaction>) -> Self {
        transactions.sort_by(|a, b| a.date.cmp(&b.date));
        Self {
            transactions: transactions.into(),
            years_carry_losses: 0,
        }
    }

    pub fn with_carry_losses(mut transactions: Vec<Transaction>, years_carry_losses: u8) -> Self {
        transactions.sort_by(|a, b| a.date.cmp(&b.date));
        Self {
            transactions: transactions.into(),
            years_carry_losses,
        }
    }

    pub fn report(&self, from: NaiveDate, to: NaiveDate) -> Money {
        let mut transactions = self.transactions.clone();
        let mut state_map: HashMap<String, (Money, isize)> = HashMap::new();
        let mut profits = Money::default();
        let last_year_carry = to.year() - (self.years_carry_losses as i32);
        let last_date_carry = to.with_year(last_year_carry).unwrap();

        while let Some(tr) = transactions.pop_front() {
            match tr.r#type() {
                TransactionType::Buy => {
                    let entry = state_map.entry(tr.isin).or_insert(Default::default());
                    entry.0.add(&tr.local_value.abs());
                    entry.1 += tr.quantity;
                }
                TransactionType::Sell => {
                    //todo
                    // if it doesn't exist in the map, it means we should return an error
                    // you cannot sell what you don't have
                    let entry = state_map.get_mut(&tr.isin).unwrap();
                    if tr.date >= from && last_date_carry <= tr.date {
                        let mut avg_price = entry.0.clone();
                        avg_price.div(entry.1);
                        avg_price.mul(tr.quantity);

                        let mut local_profit = tr.local_value.clone();
                        log::debug!("local: {:#?}  avg: {:#?}", local_profit, avg_price.abs());
                        let abs_avg = avg_price.abs();

                        local_profit.sub(&abs_avg);
                        profits.add(&local_profit);
                    }
                    entry.0.sub(&tr.local_value);
                    //todo handle error if negative
                    entry.1 += tr.quantity;
                }
            }

            if let Some(next) = transactions.get(0) {
                if next.date > to {
                    break;
                }
            }
        }
        profits.truncate_trailing_zeros()
    }
}

#[cfg(test)]
mod test {
    use crate::portfolio::Portfolio;
    use crate::{Money, Transaction};
    use chrono::{NaiveDate, NaiveTime};
    use decimal::d128;

    #[test]
    fn it_works() {
        let from = NaiveDate::from_ymd(2020, 1, 1);
        let to = NaiveDate::from_ymd(2021, 1, 1);
        let time = NaiveTime::from_hms(1, 1, 1);

        let transactions = vec![
            Transaction {
                date: from.clone(),
                time: time,
                product: "".to_string(),
                isin: "1".to_string(),
                reference: "".to_string(),
                quantity: 1,
                venue: "".to_string(),
                price: Default::default(),
                local_value: Money::new(d128::from(-500)),
                value: Default::default(),
                transaction: None,
                exchange_rate: None,
                total: "".to_string(),
                order_id: "".to_string(),
            },
            Transaction {
                date: to.clone(),
                time: time,
                product: "".to_string(),
                isin: "1".to_string(),
                reference: "".to_string(),
                quantity: 1,
                venue: "".to_string(),
                price: Default::default(),
                local_value: Money::new(d128::from(600)),
                value: Default::default(),
                transaction: None,
                exchange_rate: None,
                total: "".to_string(),
                order_id: "".to_string(),
            },
        ];

        let portfolio = Portfolio::with_carry_losses(transactions, 1);
        let report = portfolio.report(
            NaiveDate::from_ymd(2021, 1, 1),
            NaiveDate::from_ymd(2021, 12, 30),
        );

        assert_eq!(report, Money::new(d128::from(100)))
    }
}
