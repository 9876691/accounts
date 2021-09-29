## Parse transactions into accounts
 
To run the unit tests

`cargo test`

To run

`cargo run -- transactions.csv > accounts.csv`

## Testing

Unit tests check the following

1. Deposits and Withdrawals
1. Disputes and Resolve
1. Chargebacks
1. Chargebacks when no dispute was made

## Test data

Available in transactions.csv, most testing was done wth the unit tests.

## Todo

1. Better organise the code rather than all in 1 file.
1. Consider parsing the float values from the CSV into u64. Floats can give rounding errors.
1. Format the results to 4dp