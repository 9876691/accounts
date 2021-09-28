use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::io;

// This is what we pull out of the CSV
#[derive(Debug, Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    transaction_id: u32,
    // Amounts are converted from float to integer. This is so we don't
    // get issues with rounding etc.
    amount: Option<u64>,
}


#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Dispute,
    Deposit,
    Withdrawal,
    Resolve,
    Chargeback,
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
                amount: tx.amount
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
            match tx {
                &Transaction {
                    tx_type: TransactionType::Chargeback, ..
                } => {
                    let tx = self.get_disputed_transaction(tx.transaction_id);
                    match tx {
                        Some(Transaction {
                            tx_type: TransactionType::Deposit,
                            amount: Some(amount),
                            ..
                        }) => {
                            held -= amount;
                            locked = true;
                        }
                        _ => {}
                    }
                }
                
                &Transaction {
                    tx_type: TransactionType::Deposit, 
                    amount: Some(amount),
                    ..
                } =>  {
                    available += amount;
                }

                &Transaction {
                    tx_type: TransactionType::Dispute, 
                    ..
                } => {
                    let tx = self.get_disputed_transaction(tx.transaction_id);
                    match tx {
                        Some(Transaction {
                            tx_type: TransactionType::Deposit,
                            amount: Some(amount),
                            ..
                        }) => {
                            held += amount;
                            available -= amount;
                        }
                        _ => {}
                    }
                }

                &Transaction {
                    tx_type: TransactionType::Resolve, 
                    ..
                } => {
                    let tx = self.get_disputed_transaction(tx.transaction_id);
                    match tx {
                        Some(Transaction {
                            tx_type: TransactionType::Deposit,
                            amount: Some(amount),
                            ..
                        }) => {
                            held -= amount;
                            available += amount;
                        }
                        _ => {}
                    }
                }

                
                &Transaction {
                    tx_type: TransactionType::Withdrawal, 
                    amount: Some(amount),
                    ..
                } =>  {
                    if amount <= available {
                        available -= amount;
                    }
                }

                _ => {}
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
        let tx: Transaction = result?;

        accounts.add_transaction(tx);
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
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(10_5000)
        });

        let closing_balances = accounts.generate_closing_balances();

        assert_eq!(closing_balances.len(), 1);

        assert_eq!(closing_balances.get(0).unwrap().total, 10_5000);

        // Make another deposit
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 2,
            amount: Some(20_5000)
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 31_0000);

        // Make a withdrawal for more money than we have
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            transaction_id: 2,
            amount: Some(40_0000)
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 31_0000);

        // Make a withdrawal for fubnds we have
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            transaction_id: 2,
            amount: Some(10_5000)
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 20_5000);

        // Some more just in case
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 2,
            amount: Some(50_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            transaction_id: 2,
            amount: Some(40_5000)
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.get(0).unwrap().total, 30_5000);
    }

    #[test]
    fn test_multiple_clients() {

        let mut accounts: Accounts = Default::default();

        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 2,
            transaction_id: 2,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 3,
            transaction_id: 3,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 4,
            transaction_id: 4,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 5,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 6,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 7,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 8,
            amount: Some(10_5000)
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.len(), 4);
    }

    #[test]
    fn test_dispute_and_resolve() {

        let mut accounts: Accounts = Default::default();
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Dispute,
            client_id: 1,
            transaction_id: 1,
            amount: None
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.len(), 1);
        assert_eq!(closing_balances.get(0).unwrap().total, 10_5000);
        assert_eq!(closing_balances.get(0).unwrap().held, 10_5000);
        assert_eq!(closing_balances.get(0).unwrap().available, 0);

        // Keep adding money see what happens
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 3,
            amount: Some(10_5000)
        });
        accounts.add_transaction(Transaction {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 4,
            amount: Some(10_5000)
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
            amount: None
        });

        let closing_balances = accounts.generate_closing_balances();
        assert_eq!(closing_balances.len(), 1);
        assert_eq!(closing_balances.get(0).unwrap().total, 31_5000);
        assert_eq!(closing_balances.get(0).unwrap().held, 0);
        assert_eq!(closing_balances.get(0).unwrap().available, 31_5000);
    }
}
