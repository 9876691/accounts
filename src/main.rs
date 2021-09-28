use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::io;

// This is what we pull out of the CSV
#[derive(Debug, Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    tx_type: String,
    client: u16,
    tx: u32,
    amount: Option<f32>,
}

// We'll tighten up the data model using an enum, then the compiler
// can help us as we don't have to do stribng compares on transaction type.
struct Transaction {
    tx_type: TransactionType,
    client_id: u16,
    transaction_id: u32,
}

// Amounts are converted from float to integer. This is so we don't
// get issues with rounding etc.
#[derive(Clone)]
enum TransactionType {
    Dispute,
    Deposit { amount: u64 },
    Withdrawal { amount: u64 },
    Resolve,
    Chargeback,
}

// Convert the transaction we get out of the CSV into our Transaction and enum
impl Transaction {
    fn from(tx: CsvTransaction) -> Option<Transaction> {
        match tx {
            CsvTransaction {
                amount: Some(amount),
                ..
            } if tx.tx_type == "deposit" => Some(Transaction {
                tx_type: TransactionType::Deposit {
                    amount: (amount * 1000.0) as u64,
                },
                client_id: tx.client,
                transaction_id: tx.tx,
            }),

            CsvTransaction { .. } if tx.tx_type == "dispute" => Some(Transaction {
                tx_type: TransactionType::Dispute,
                client_id: tx.client,
                transaction_id: tx.tx,
            }),
            CsvTransaction {
                amount: Some(amount),
                ..
            } if tx.tx_type == "withdrawal" => Some(Transaction {
                tx_type: TransactionType::Withdrawal {
                    amount: (amount * 1000.0) as u64,
                },
                client_id: tx.client,
                transaction_id: tx.tx,
            }),

            CsvTransaction { .. } if tx.tx_type == "chargeback" => Some(Transaction {
                tx_type: TransactionType::Chargeback,
                client_id: tx.client,
                transaction_id: tx.tx,
            }),

            CsvTransaction { .. } if tx.tx_type == "resolve" => Some(Transaction {
                tx_type: TransactionType::Resolve,
                client_id: tx.client,
                transaction_id: tx.tx,
            }),
            _ => None,
        }
    }
}

// An account has a bunch of transactions
struct Account {
    id: u16,
    transactions: Vec<Transaction>,
}

impl Account {
    fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    fn get_disputed_transaction(&self, tx_id: u32) -> Option<Transaction> {
        let tx = self
            .transactions
            .iter()
            .find(|tx| tx.transaction_id == tx_id);

        if let Some(tx) = tx {
            return Some(Transaction {
                tx_type: tx.tx_type.clone(),
                client_id: tx.client_id,
                transaction_id: tx.transaction_id,
            });
        }

        return None;
    }
}

// Our accoubnt database
struct Accounts {
    accounts: HashMap<u16, Account>,
}

impl Default for Accounts {
    fn default() -> Self {
        Accounts {
            accounts: HashMap::new(),
        }
    }
}

// Implement the ability to add transactions to our accounts and will
// also implement the functionality to get the final balances.
impl Accounts {
    fn add_transaction(&mut self, tx: Transaction) {
        if let Some(account) = self.accounts.get_mut(&tx.client_id) {
            account.add_transaction(tx);
        } else {
            self.accounts.insert(
                tx.client_id,
                Account {
                    id: tx.client_id,
                    transactions: vec![tx],
                },
            );
        }
    }

    fn generate_closing_balances(&self) -> Vec<ClosingBalance> {
        self.accounts
            .iter()
            .map(|(_, account)| account.closing_balance())
            .collect()
    }
}

#[derive(Debug)]
struct ClosingBalance {
    held: u64,
    available: u64,
    total: u64,
    locked: bool,
}

impl Account {
    fn closing_balance(&self) -> ClosingBalance {
        let mut held: u64 = 0;
        let mut available: u64 = 0;
        let mut locked: bool = false;

        // The logic for running through transaction and updating held and available.
        for tx in &self.transactions {
            match tx.tx_type {
                TransactionType::Chargeback => {}
                TransactionType::Deposit { amount } => {
                    available += amount;
                }
                TransactionType::Dispute => {
                    let tx = self.get_disputed_transaction(tx.transaction_id);
                    match tx {
                        Some(Transaction {
                            tx_type: TransactionType::Withdrawal { amount },
                            ..
                        }) => {
                            held -= amount;
                            available += amount;
                        }
                        Some(Transaction {
                            tx_type: TransactionType::Deposit { amount },
                            ..
                        }) => {
                            held += amount;
                            available -= amount;
                        }
                        _ => {}
                    }
                }
                // A resolve represents a resolution to a dispute, releasing the associated held funds. 
                // Funds that were previously disputed are no longer disputed. This means that the clients 
                // held funds should decrease by the amount no longer disputed, their available funds should 
                // increase by the amount no longer disputed, and their total funds should remain the same.
                TransactionType::Resolve => {
                    let tx = self.get_disputed_transaction(tx.transaction_id);
                    match tx {
                        Some(Transaction {
                            tx_type: TransactionType::Withdrawal { amount },
                            ..
                        }) => {
                            held += amount;
                            available -= amount;
                        }
                        Some(Transaction {
                            tx_type: TransactionType::Deposit { amount },
                            ..
                        }) => {
                            held -= amount;
                            available += amount;
                        }
                        _ => {}
                    }
                }
                TransactionType::Withdrawal { amount } => {
                    if amount <= available {
                        available -= amount;
                    }
                }
            }
        }

        ClosingBalance {
            held,
            available,
            total: available + held,
            locked,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut accounts: Accounts = Default::default();

    let mut rdr = csv::Reader::from_reader(io::stdin());
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: CsvTransaction = result?;

        if let Some(tx) = Transaction::from(record) {
            accounts.add_transaction(tx);
        } else {
            dbg!("Couldn't parse it.");
        }
    }

    let closing_balances = accounts.generate_closing_balances();

    dbg!(closing_balances);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposits_and_withdrawals() {

        let mut accounts: Accounts = Default::default();

        // Make an inital deposit
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 1,
        });

        let closing_balances = accounts.generate_closing_balances();

        assert_eq!(closing_balances.len(), 1);

        assert_eq!(closing_balances.get(0).unwrap().total, 10_5000);

        // Make another deposit
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 20_5000 },
            client_id: 1,
            transaction_id: 2,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 31_0000);

        // Make a withdrawal for more money than we have
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Withdrawal{ amount: 40_0000 },
            client_id: 1,
            transaction_id: 2,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 31_0000);

        // Make a withdrawal for fubnds we have
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Withdrawal{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 2,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 20_5000);

        // Some more just in case
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 50_5000 },
            client_id: 1,
            transaction_id: 2,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Withdrawal{ amount: 40_5000 },
            client_id: 1,
            transaction_id: 2,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 30_5000);
    }

    #[test]
    fn test_multiple_clients() {

        let mut accounts: Accounts = Default::default();

        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 1,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 2,
            transaction_id: 2,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 3,
            transaction_id: 3,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 4,
            transaction_id: 4,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 5,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 6,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 7,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 8,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.len(), 4);
    }

    #[test]
    fn test_dispute_and_resolve() {

        let mut accounts: Accounts = Default::default();
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 1,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 1,
            transaction_id: 1,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.len(), 1);
        assert_eq!(closing_balances.get(0).unwrap().total, 10_5000);
        assert_eq!(closing_balances.get(0).unwrap().held, 10_5000);
        assert_eq!(closing_balances.get(0).unwrap().available, 0);

        // Keep adding money see what happens
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 3,
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit{ amount: 10_5000 },
            client_id: 1,
            transaction_id: 4,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.len(), 1);
        assert_eq!(closing_balances.get(0).unwrap().total, 31_5000);
        assert_eq!(closing_balances.get(0).unwrap().held, 10_5000);
        assert_eq!(closing_balances.get(0).unwrap().available, 21_0000);

        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Resolve,
            client_id: 1,
            transaction_id: 1,
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.len(), 1);
        assert_eq!(closing_balances.get(0).unwrap().total, 31_5000);
        assert_eq!(closing_balances.get(0).unwrap().held, 0);
        assert_eq!(closing_balances.get(0).unwrap().available, 31_5000);
    }
}
