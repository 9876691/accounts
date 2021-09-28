use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::io;

#[derive(Debug, Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    tx_type: String,
    client: u16,
    tx: u32,
    amount: Option<f32>,
}

struct Transaction {
    tx_type: TransactionType,
    client_id: u16,
    transaction_id: u32,
}

#[derive(Clone)]
enum TransactionType {
    Dispute,
    Deposit { amount: u64 },
    Withdrawal { amount: u64 },
    Resolve,
    Chargeback,
}

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
                TransactionType::Resolve => {}
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
