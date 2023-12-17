use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use subxt::tx::{Signer, TxPayload};
use subxt_signer::{
    sr25519::{Keypair, Seed},
    DeriveJunction,
};

mod metadata;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(long)]
    endpoint: Option<String>,
    #[clap(long, required = true)]
    private_key: String,
    #[clap(long, required = true)]
    remark: String,
    #[clap(long, required = true)]
    chain: String,
    /// Specifies how many transactions to fill the mempool with.
    #[clap(long, short = 'n', default_value = "100")]
    inflight_num: usize,
}

impl Cli {
    fn ensure_kusama(&self) -> Result<()> {
        if self.chain != "kusama" {
            bail!("Only Kusama is supported");
        }
        Ok(())
    }

    fn private_key(&self) -> Result<Keypair> {
        // strip 0x prefix
        let private_key = if self.private_key.starts_with("0x") {
            &self.private_key[2..]
        } else {
            &self.private_key
        };
        let raw = hex::decode(&private_key).map_err(|e| anyhow!(e))?;
        let mut seed: Seed = Seed::default();
        if raw.len() != seed.len() {
            bail!(
                "Keyfile length invalid, expected {} bytes, got {} bytes",
                seed.len(),
                raw.len()
            );
        }
        seed.copy_from_slice(&raw[..]);
        Ok(Keypair::from_seed(seed)?)
    }
}

/// Takes a keypair and derives a new keypair from it.
///
/// Calling this function with the same parameters must return the same result.
fn derive_account(keypair: &Keypair, index: u32) -> Keypair {
    keypair.derive([DeriveJunction::hard(index)])
}

struct Database {
    sqlite: sqlx::SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let sqlite = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite://inscribed.sqlite")
            .await?;
        Ok(Self { sqlite })
    }

    /// Returns the next usable index for deriving an account.
    pub async fn get_next_index(&self) -> Result<u32> {
        let row = sqlx::query!("SELECT value FROM kv WHERE key = 'next_index'")
            .fetch_one(&self.sqlite)
            .await?;
        let value = row
            .value
            .expect("next_index should be present")
            .parse::<u32>()?;
        Ok(value)
    }

    pub async fn get_pending_txns(&self) -> Result<Vec<Txn>> {
        let rows = sqlx::query!("SELECT extrinsic_data FROM txns")
            .fetch_all(&self.sqlite)
            .await?;
        let txns = rows
            .into_iter()
            .map(|row| Txn(row.extrinsic_data))
            .collect();
        Ok(txns)
    }

    pub async fn update(&self, new_next_index: u32, txns: Vec<Txn>) -> Result<()> {
        let mut tx = self.sqlite.begin().await?;
        sqlx::query!(
            "UPDATE kv SET value = ? WHERE key = 'next_index'",
            new_next_index
        )
        .execute(&mut *tx)
        .await?;
        for txn in txns {
            sqlx::query!("INSERT INTO txns (extrinsic_data) VALUES (?)", txn.0)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

struct AccountState {
    keypair: Keypair,
    nonce: u64,
}

#[derive(Clone)]
struct Txn(Vec<u8>);

#[derive(Clone)]
struct Rpc {
    client: metadata::kusama::Client,
}

impl Rpc {
    pub async fn new(endpoint: &str) -> Result<Self> {
        let client = metadata::kusama::new_client(endpoint).await?;
        Ok(Self { client })
    }

    pub async fn get_nonce(&self, keypair: &Keypair) -> Result<u64> {
        let account_id =
            <subxt_signer::sr25519::Keypair as Signer<metadata::kusama::Config>>::account_id(
                keypair,
            );
        let nonce = self.client.tx().account_nonce(&account_id).await?;
        Ok(nonce)
    }

    pub fn sign_uxt(&self, keypair: &Keypair, nonce: u64, uxt: impl TxPayload) -> Result<Txn> {
        let signed =
            self.client
                .tx()
                .create_signed_with_nonce(&uxt, keypair, nonce, Default::default())?;
        Ok(Txn(signed.into_encoded()))
    }

    pub async fn submit(&self, txn: Txn) -> Result<()> {
        let _ = self.client.backend().submit_transaction(&txn.0).await?;
        Ok(())
    }
}

fn sign_transfer_all(rpc: &Rpc, sender: &AccountState, receiver: &AccountState) -> Result<Txn> {
    let xfer_uxt = metadata::kusama::tx().balances().transfer_all(
        subxt::utils::MultiAddress::Id(<subxt_signer::sr25519::Keypair as Signer<
            metadata::kusama::Config,
        >>::account_id(&receiver.keypair)),
        false,
    );
    let signed = rpc.sign_uxt(&sender.keypair, sender.nonce, xfer_uxt)?;
    Ok(signed)
}

fn sign_mint(rpc: &Rpc, minter: &AccountState, remark: Vec<u8>) -> Result<Txn> {
    let uxt = metadata::kusama::tx().system().remark_with_event(remark);
    let signed = rpc.sign_uxt(&minter.keypair, minter.nonce, uxt)?;
    Ok(signed)
}

fn sign_transfer_and_mint(
    rpc: &Rpc,
    prev: &AccountState,
    next: &AccountState,
    remark: Vec<u8>,
) -> Result<Vec<Txn>> {
    Ok(vec![
        sign_transfer_all(rpc, prev, next)?,
        sign_mint(rpc, next, remark)?,
    ])
}

/// The service that makes sure that it keeps the mempool filled with transactions.
struct SubmissionService {
    semaphore: tokio::sync::Semaphore,
    rpc: Rpc,
}

impl SubmissionService {
    pub fn new(inflight_num: usize, rpc: Rpc) -> Self {
        Self {
            semaphore: tokio::sync::Semaphore::new(inflight_num),
            rpc,
        }
    }

    /// Blocks until there is a slot available in the mempool.
    pub async fn submit(&self, txn: Txn) -> Result<()> {
        let _permit = self.semaphore.acquire().await;
        self.rpc.submit(txn).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // first CLI stuff. Ensure it's all correct.
    cli.ensure_kusama()?;
    let root_keypair = Arc::new(cli.private_key()?);

    println!("remark: {}", &cli.remark);

    let endpoint = metadata::kusama::pick_endpoint(cli.endpoint.as_deref());
    println!("connecting to {}", &endpoint);
    let rpc = Rpc::new(&endpoint).await?;
    let remark = Arc::new(cli.remark.as_bytes().to_vec());

    let db = Database::new().await?;
    let submission_service = SubmissionService::new(cli.inflight_num, rpc.clone());
    let pending = db.get_pending_txns().await?;

    // This will block and thus can lead to a dead lock if we don't purge submission service.
    for txn in pending {
        submission_service.submit(txn).await?;
    }

    let mut index = db.get_next_index().await?;
    loop {
        let prev = if index == 0 {
            // first time, use the root keypair. Request the nonce.
            let keypair = Keypair::clone(&root_keypair);
            let nonce = rpc.get_nonce(&keypair).await?;
            AccountState { keypair, nonce }
        } else {
            // otherwise, derive the keypair from the previous one. The nonce must be 1 because the
            // previous account should've submitted the mint transaction.
            let keypair = derive_account(&root_keypair, index - 1);
            AccountState { keypair, nonce: 1 }
        };
        let next = {
            // Next always has nonce 0.
            let keypair = derive_account(&root_keypair, index);
            AccountState { keypair, nonce: 0 }
        };
        let txns = sign_transfer_and_mint(&rpc, &prev, &next, Vec::clone(&remark))?;

        index += 1;
        db.update(index, txns.clone()).await?;

        for txn in txns {
            tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
            submission_service.submit(txn).await?;
        }
    }
}
