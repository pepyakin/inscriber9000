// Generated via:
//
// subxt codegen --url wss://kusama.api.onfinality.io/public-ws | rustfmt \
//     --edition=2021 --emit=stdout > src/metadata/kusama.rs
mod kusama_gen;

pub mod kusama {
    pub use super::kusama_gen::api::*;

    pub type Config = subxt::SubstrateConfig;
    pub type Client = subxt::OnlineClient<Config>;

    const ENDPOINTS: &[&str] = &[
        "wss://rpc.dotters.network/kusama",
        "wss://rpc-kusama.luckyfriday.io",
        "wss://kusama.api.onfinality.io/public-ws",
        "wss://kusama.public.curie.radiumblock.co/ws",
        "wss://ksm-rpc.stakeworld.io",
    ];

    pub fn pick_endpoint(endpoint: Option<&str>) -> &str {
        use rand::Rng;
        match endpoint {
            Some(endpoint) => endpoint,
            None => {
                let mut rng = rand::thread_rng();
                let index = rng.gen_range(0..ENDPOINTS.len());
                ENDPOINTS[index]
            }
        }
    }

    pub async fn new_client(endpoint: &str) -> anyhow::Result<Client> {
        let client = Client::from_url(endpoint).await?;
        Ok(client)
    }
}
