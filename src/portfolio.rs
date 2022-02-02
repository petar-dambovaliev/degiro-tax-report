use crate::money::Error;
use crate::{Money, Transaction, TransactionType};
use anyhow::anyhow;
use chrono::Datelike;
use futures::stream::Stream;
use futures::{pin_mut, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;

#[derive(Clone, Debug)]
pub struct Report {
    profits: HashMap<i32, (Money, Money)>,
    years_carry_losses: u8,
    year: i32,
}

impl Report {
    ///returns the total profits
    pub fn profit(&self) -> anyhow::Result<Money> {
        let profit = self.profits.get(&self.year).ok_or(anyhow!(
            "cannot find data for year: {} profits: {:#?}",
            self.year,
            self.profits
        ))?;

        let mut res = profit.0.clone();
        res.add(&profit.1)?;
        Ok(res)
    }

    /// returns the profits minus the carry over losses
    /// from previous years
    pub fn adjusted_profit(&self) -> Result<Money, Error> {
        let mut prf: Vec<(i32, (Money, Money))> = self
            .profits
            .iter()
            .filter(|(k, _)| **k >= self.year - self.years_carry_losses as i32)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        prf.sort_by(|a, b| a.0.cmp(&b.0));

        let mut start = false;
        let mut total = Money::default();
        let mut profit = Money::default();

        for (year, prof) in prf {
            let mut add = prof.0.clone();
            add.add(&prof.1)?;
            if year == self.year {
                profit = add;
                break;
            }
            if !start && add.is_negative() {
                start = true;
            }

            if start {
                total.add(&add)?;

                if !total.is_negative() {
                    total = Money::default();
                    start = false;
                }
            }
        }

        profit.add(&total)?;
        Ok(profit.truncate_trailing_zeros())
    }
}

pub struct Portfolio<S: Stream<Item = anyhow::Result<Transaction>>> {
    tr_stream: S,
    years_carry_losses: u8,
}

impl<S: Stream<Item = anyhow::Result<Transaction>>> Portfolio<S> {
    pub fn new(tr_stream: S) -> Self {
        Self {
            tr_stream,
            years_carry_losses: 0,
        }
    }

    pub fn with_carry_losses(tr_stream: S, years_carry_losses: u8) -> Self {
        Self {
            tr_stream,
            years_carry_losses,
        }
    }

    fn calc_trans_profit(tr: &Transaction, entry: &State) -> Result<Money, Error> {
        let mut avg_price = entry.avg.clone();
        avg_price.mul(tr.quantity)?;

        let mut local_profit = tr.value.clone();
        let abs_avg = avg_price.abs();
        local_profit.sub(&abs_avg)?;
        Ok(local_profit)
    }

    pub async fn report(self, year: i32) -> anyhow::Result<Report> {
        let tr_peek = self.tr_stream.peekable();
        pin_mut!(tr_peek);
        let mut tr_pin: Pin<&mut _> = tr_peek;

        let mut state_map: HashMap<String, State> = HashMap::new();
        let mut profits = HashMap::new();

        while let Some(tr) = tr_pin.as_mut().next().await {
            let tr = tr?;
            if let Some(next_tr) = tr_pin.as_mut().peek().await {
                match next_tr {
                    Ok(nt) => {
                        assert!(tr.date <= nt.date, "tr: {:#?} next: {:#?}", tr, next_tr);
                    }
                    Err(e) => return Err(anyhow!("{}", e)),
                }
            }

            match tr.r#type() {
                TransactionType::Buy => {
                    let entry = state_map.entry(tr.isin).or_insert(Default::default());
                    entry.total.add(&tr.value.abs())?;
                    entry.qty += tr.quantity;

                    let mut avg_price = entry.total.clone();
                    avg_price.div(entry.qty)?;

                    entry.avg = avg_price;
                }
                TransactionType::Sell => {
                    assert_ne!(tr.quantity, 0);
                    let entry = state_map.get_mut(&tr.isin).unwrap();
                    let local_profit = Self::calc_trans_profit(&tr, &entry)?;
                    let profit = profits
                        .entry(tr.date.year())
                        .or_insert((Money::default(), Money::default()));

                    if local_profit.is_negative() {
                        profit.1.add(&local_profit)?;
                    } else {
                        profit.0.add(&local_profit)?;
                    }

                    assert!(!tr.value.is_negative());
                    entry.total.sub(&tr.value)?;
                    entry.qty += tr.quantity;
                }
            }

            if let Some(next) = tr_pin.as_mut().peek().await {
                match next {
                    Ok(nt) => {
                        if nt.date.year() > year {
                            break;
                        }
                    }
                    Err(e) => return Err(anyhow!("{}", e)),
                }
            }
        }

        Ok(Report {
            profits,
            years_carry_losses: self.years_carry_losses,
            year,
        })
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
    use chrono::NaiveDate;
    use decimal::d128;
    use futures::stream;

    #[tokio::test]
    async fn losses_carry_over() {
        let from = NaiveDate::from_ymd(2020, 1, 1);
        let to = NaiveDate::from_ymd(2021, 1, 1);

        let transactions = vec![
            Ok(Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                to.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            )),
        ];

        let portfolio = Portfolio::with_carry_losses(stream::iter(transactions), 1);
        let report = portfolio.report(2021).await.unwrap();

        assert_eq!(
            report.adjusted_profit().unwrap(),
            Money::new(d128::from(-100))
        )
    }

    #[tokio::test]
    async fn losses_carry_over_different_isin() {
        let from = NaiveDate::from_ymd(2020, 1, 1);
        let to = NaiveDate::from_ymd(2021, 1, 1);

        let transactions = vec![
            Ok(Transaction::new_unchecked(
                from.clone(),
                "2".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                to.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                to.clone(),
                "2".to_string(),
                -1,
                Money::new(d128::from(500_i32)),
                "id".to_string(),
            )),
        ];

        let portfolio = Portfolio::with_carry_losses(stream::iter(transactions), 1);
        let report = portfolio.report(2021).await.unwrap();

        assert_eq!(
            report.adjusted_profit().unwrap(),
            Money::new(d128::from(-100))
        )
    }

    #[tokio::test]
    async fn losses_carry_over_different_isin_multiple_years() {
        let from = NaiveDate::from_ymd(2020, 1, 1);
        let to = NaiveDate::from_ymd(2021, 1, 1);

        let transactions = vec![
            Ok(Transaction::new_unchecked(
                from.clone(),
                "2".to_string(),
                1,
                Money::new(d128::from(-500_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                2,
                Money::new(d128::from(-1000_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                from.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                to.clone(),
                "1".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            )),
            Ok(Transaction::new_unchecked(
                to.clone(),
                "2".to_string(),
                -1,
                Money::new(d128::from(400_i32)),
                "id".to_string(),
            )),
        ];

        let portfolio = Portfolio::with_carry_losses(stream::iter(transactions), 2);
        let report = portfolio.report(2021).await.unwrap();

        assert_eq!(
            report.adjusted_profit().unwrap(),
            Money::new(d128::from(-300))
        )
    }
}
