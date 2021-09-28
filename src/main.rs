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

enum TransactionType {
    Dispute,
    Deposit,
    Withdrawal,
    Resolve,
    Chargeback,
}

impl Transaction {
    fn from(tx: CsvTransaction) -> Option<Transaction> {
        match tx.tx_type.as_str() {
            "deposit" => Some(Transaction {
                tx_type: TransactionType::Deposit,
                client_id: tx.client,
                transaction_id: tx.tx,
            }),
            "dispute" => Some(Transaction {
                tx_type: TransactionType::Dispute,
                client_id: tx.client,
                transaction_id: tx.tx,
            }),
            "withdrawal" => Some(Transaction {
                tx_type: TransactionType::Chargeback,
                client_id: tx.client,
                transaction_id: tx.tx,
            }),
            "chargeback" => Some(Transaction {
                tx_type: TransactionType::Deposit,
                client_id: tx.client,
                transaction_id: tx.tx,
            }),
            "resolve" => Some(Transaction {
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
    fn generate_closing_balances(&self) {}
}

struct ClosingBalance {
    held: u32,
    available: u32,
    total: u32,
    locked: bool,
}

impl Account {
    fn closing_balance() -> ClosingBalance {
        ClosingBalance {
            held: 0,
            available: 0,
            total: 0,
            locked: false,
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
