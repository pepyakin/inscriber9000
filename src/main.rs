use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use metadata::kusama::system::calls::types::RemarkWithEvent;
use subxt::tx::{Payload, Signer};
use subxt_signer::sr25519::{Keypair, Seed};

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // first CLI stuff. Ensure it's all correct.
    cli.ensure_kusama()?;
    let keypair = Arc::new(cli.private_key()?);

    println!("remark: {}", &cli.remark);

    let endpoint = metadata::kusama::pick_endpoint(cli.endpoint.as_deref());
    println!("connecting to {}", &endpoint);
    let client = metadata::kusama::new_client(endpoint).await?;
    let remark = cli.remark.as_bytes().to_vec();
    let mut nonce = client
        .tx()
        .account_nonce(&<subxt_signer::sr25519::Keypair as Signer<
            metadata::kusama::Config,
        >>::account_id(&keypair))
        .await?;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(cli.inflight_num));
    loop {
        tokio::spawn({
            let permit = semaphore.clone().acquire_owned().await;
            let client = client.clone();
            let xt = metadata::kusama::tx()
                .system()
                .remark_with_event(remark.clone());
            let keypair = keypair.clone();
            async move {
                // move permit in.
                let _permit = permit;
                do_it(&client, nonce, &xt, &keypair)
                    .await
                    .unwrap_or_else(|e| {
                        eprintln!("{}: {:?}", nonce, e);
                    });

                async fn do_it(
                    client: &metadata::kusama::Client,
                    nonce: u64,
                    xt: &Payload<RemarkWithEvent>,
                    keypair: &Keypair,
                ) -> Result<()> {
                    let tx = client.tx().create_signed_with_nonce(
                        xt,
                        keypair,
                        nonce,
                        Default::default(),
                    )?;
                    let mut tx_status = tx.submit_and_watch().await?;
                    while let Some(status) = tx_status.next().await {
                        let status = status?;
                        match status {
                            subxt::tx::TxStatus::InBestBlock(tx)
                            | subxt::tx::TxStatus::InFinalizedBlock(tx) => {
                                let extrinsic_hash = tx.extrinsic_hash();
                                println!("{}: {:?}", nonce, extrinsic_hash);
                                break;
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                }
            }
        });
        nonce += 1;
    }
}
