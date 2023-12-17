// Generated via:
//
// subxt codegen --url wss://kusama.api.onfinality.io/public-ws | rustfmt \
//     --edition=2021 --emit=stdout > src/metadata/kusama.rs
mod kusama_gen;

pub mod kusama {
    pub use super::kusama_gen::api::*;

    pub type Config = subxt::SubstrateConfig;
    pub type Client = subxt::OnlineClient<Config>;
}
