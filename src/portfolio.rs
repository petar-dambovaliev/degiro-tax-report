use crate::{Money, Transaction, TransactionType};
use chrono::{Datelike, NaiveDate};
use std::collections::{HashMap, VecDeque};

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

    fn should_carry_losses(&self, date: &NaiveDate, first: &NaiveDate, last: &NaiveDate) -> bool {
        self.years_carry_losses > 0 && first <= date && last >= date
    }

    fn calc_trans_profit(&self, tr: &Transaction, entry: &State) -> Money {
        let mut avg_price = entry.avg.clone();
        avg_price.mul(tr.quantity);

        let mut local_profit = tr.value.clone();
        let abs_avg = avg_price.abs();
        local_profit.sub(&abs_avg);
        local_profit
    }

    pub fn report(&self, from: NaiveDate, to: NaiveDate) -> Money {
        let mut transactions = self.transactions.clone();
        let mut state_map: HashMap<String, State> = HashMap::new();
        let mut profits = Money::default();
        let last_year_carry = to.year() - (self.years_carry_losses as i32);
        let first_date_carry = to.with_year(last_year_carry).unwrap();
        let last_date_carry = to.with_year(to.year() - 1).unwrap();
        let mut carry_losses = Money::default();

        while let Some(tr) = transactions.pop_front() {
            match tr.r#type() {
                TransactionType::Buy => {
                    let entry = state_map.entry(tr.isin).or_insert(Default::default());
                    entry.total.add(&tr.value.abs());
                    entry.qty += tr.quantity;

                    let mut avg_price = entry.total.clone();
                    avg_price.div(entry.qty);

                    entry.avg = avg_price;
                }
                TransactionType::Sell => {
                    if tr.quantity == 0 {
                        panic!("already sold out before")
                    }
                    //todo
                    // if it doesn't exist in the map, it means we should return an error
                    // you cannot sell what you don't have
                    let entry = state_map.get_mut(&tr.isin).unwrap();
                    if tr.date >= from && tr.date <= to {
                        let local_profit = self.calc_trans_profit(&tr, &entry);
                        profits.add(&local_profit);
                    } else if self.should_carry_losses(
                        &tr.date,
                        &first_date_carry,
                        &last_date_carry,
                    ) {
                        let local_profit = self.calc_trans_profit(&tr, &entry);
                        //accumulate total for carry over previous years
                        carry_losses.add(&local_profit);
                    }

                    entry.total.sub(&tr.value);
                    //todo handle error if negative
                    entry.qty += tr.quantity;
                }
            }

            if let Some(next) = transactions.get(0) {
                if next.date > to {
                    break;
                }
            }
        }

        if self.years_carry_losses > 0 && carry_losses.is_negative() {
            profits.add(&carry_losses);
        }

        profits.truncate_trailing_zeros()
    }
}

#[derive(Debug, Default)]
struct State {
    total: Money,
    avg: Money,
    qty: isize,
}

#[cfg(test)]
mod test {
    use crate::portfolio::Portfolio;
    use crate::{Money, Transaction};
    use chrono::{NaiveDate, NaiveTime};
    use decimal::d128;

    #[test]
    fn losses_carry_over() {
        let from = NaiveDate::from_ymd(2020, 1, 1);
        let to = NaiveDate::from_ymd(2021, 1, 1);
        let time = NaiveTime::from_hms(1, 1, 1);

        let transactions = vec![
            Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                to.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            ),
        ];

        let portfolio = Portfolio::with_carry_losses(transactions, 1);
        let report = portfolio.report(
            NaiveDate::from_ymd(2021, 1, 1),
            NaiveDate::from_ymd(2021, 12, 30),
        );

        assert_eq!(report, Money::new(d128::from(-100)))
    }

    #[test]
    fn losses_carry_over_different_isin() {
        let from = NaiveDate::from_ymd(2020, 1, 1);
        let to = NaiveDate::from_ymd(2021, 1, 1);
        let time = NaiveTime::from_hms(1, 1, 1);

        let transactions = vec![
            Transaction::new_unchecked(
                from.clone(),
                "2".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                to.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                to.clone(),
                "2".to_string(),
                -1,
                Money::new(d128::from(500_i32)),
                "id".to_string(),
            ),
        ];

        let portfolio = Portfolio::with_carry_losses(transactions, 1);
        let report = portfolio.report(
            NaiveDate::from_ymd(2021, 1, 1),
            NaiveDate::from_ymd(2021, 12, 30),
        );

        assert_eq!(report, Money::new(d128::from(-100)))
    }

    #[test]
    fn losses_carry_over_different_isin_multiple_years() {
        let from = NaiveDate::from_ymd(2020, 1, 1);
        let to = NaiveDate::from_ymd(2021, 1, 1);
        let time = NaiveTime::from_hms(1, 1, 1);

        let transactions = vec![
            Transaction::new_unchecked(
                from.clone(),
                "2".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                2,
                Money::new(d128::from(-1000_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                to.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            ),
            Transaction::new_unchecked(
                to.clone(),
                "2".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            ),
        ];

        let portfolio = Portfolio::with_carry_losses(transactions, 2);
        let report = portfolio.report(
            NaiveDate::from_ymd(2021, 1, 1),
            NaiveDate::from_ymd(2021, 12, 30),
        );

        assert_eq!(report, Money::new(d128::from(-300)))
    }
}
