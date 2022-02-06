Library and a cli application to calculate annual tax reports for capital gains

```
// tells you how much realized gains you had for the year
// irrelevant of losses for previous years
cargo run --  -f ./testdata/data.csv -y 2021 -c 5  unadjusted
```

```
// this one takes into account losses from previous years
// in this case, 5
cargo run --  -f ./testdata/data.csv -y 2021 -c 5  adjusted
```