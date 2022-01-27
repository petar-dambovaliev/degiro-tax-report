use crate::{Money, Transaction, TransactionType};
use chrono::Datelike;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct Report {
    profits: HashMap<i32, (Money, Money)>,
    years_carry_losses: u8,
    year: i32,
}

impl Report {
    ///returns the total profits
    pub fn profit(&self) -> Money {
        let profit = self.profits.get(&self.year).unwrap();
        let mut res = profit.0.clone();
        res.add(&profit.1);
        res
    }

    /// returns the profits minus the carry over losses
    /// from previous years
    pub fn adjusted_profit(&self) -> Money {
        let mut prf: Vec<(i32, (Money, Money))> = self
            .profits
            .iter()
            .filter(|(k, v)| **k >= self.year - self.years_carry_losses as i32)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        prf.sort_by(|a, b| a.0.cmp(&b.0));

        let mut start = false;
        let mut total = Money::default();
        let mut profit = Money::default();

        for (year, prof) in prf {
            let mut add = prof.0.clone();
            add.add(&prof.1);
            if year == self.year {
                profit = add;
                break;
            }
            if !start && add.is_negative() {
                start = true;
            }

            if start {
                total.add(&add);

                if !total.is_negative() {
                    total = Money::default();
                    start = false;
                }
            }
        }

        profit.add(&total);
        profit.truncate_trailing_zeros()
    }
}

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

    fn calc_trans_profit(&self, tr: &Transaction, entry: &State) -> Money {
        let mut avg_price = entry.avg.clone();
        avg_price.mul(tr.quantity);

        let mut local_profit = tr.value.clone();
        let abs_avg = avg_price.abs();
        local_profit.sub(&abs_avg);
        local_profit
    }

    pub fn report(&self, year: i32) -> Report {
        let mut transactions = self.transactions.clone();
        let mut state_map: HashMap<String, State> = HashMap::new();
        let mut profits = HashMap::new();

        while let Some(tr) = transactions.pop_front() {
            if let Some(next_tr) = transactions.front() {
                assert!(
                    tr.date <= next_tr.date,
                    "tr: {:#?} next: {:#?}",
                    tr,
                    next_tr
                );
            }

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
                    assert_ne!(tr.quantity, 0);
                    let entry = state_map.get_mut(&tr.isin).unwrap();
                    let local_profit = self.calc_trans_profit(&tr, &entry);
                    let profit = profits
                        .entry(tr.date.year())
                        .or_insert((Money::default(), Money::default()));

                    if local_profit.is_negative() {
                        profit.1.add(&local_profit);
                    } else {
                        profit.0.add(&local_profit);
                    }

                    assert!(!tr.value.is_negative());
                    entry.total.sub(&tr.value);
                    entry.qty += tr.quantity;
                }
            }

            if let Some(next) = transactions.front() {
                if next.date.year() > year {
                    break;
                }
            }
        }

        Report {
            profits,
            years_carry_losses: self.years_carry_losses,
            year,
        }
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
        let report = portfolio.report(2021);

        assert_eq!(report.adjusted_profit(), Money::new(d128::from(-100)))
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
        let report = portfolio.report(2021);

        assert_eq!(report.adjusted_profit(), Money::new(d128::from(-100)))
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
        let report = portfolio.report(2021);

        assert_eq!(report.adjusted_profit(), Money::new(d128::from(-300)))
    }
}
