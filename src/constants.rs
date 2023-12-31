use std::{collections::HashSet, str::FromStr};

use ethers_core::{
    k256::ecdsa::SigningKey,
    rand::thread_rng,
    types::{Bytes, H160, H256},
    utils::keccak256,
};
use ethers_providers::Provider;
use ethers_signers::{LocalWallet, Signer, Wallet};
use jsonrpsee::http_client::{transport, HttpClientBuilder};
use lazy_static::lazy_static;
use mev_share_rpc_api::FlashbotsSignerLayer;
use mev_share_sse::EventClient;
use tower::ServiceBuilder;

use crate::{client::Client, progress::Progress};

pub const SSE: &str = "https://mev-share-goerli.flashbots.net";
pub const RELAY: &str = "https://relay-goerli.flashbots.net:443";

pub const MAX_GAS_PRICE: u128 = 100;
pub const MAX_PRIORITY_FEE: u128 = 100;
pub const GAS_LIMIT: u128 = 500000;
pub const GWEI: u128 = 1000000000;
pub const TIP: u128 = 10 * GWEI;

pub const BUNDLE_BLOCK_WINDOW: u64 = 5;

lazy_static! {
    pub static ref RPC_URL: String = dotenv::vars()
        .find(|e| e.0 == "RPC")
        .expect("Cannot find RPC URL in ENV")
        .1;
    pub static ref WS_URL: String = dotenv::vars()
        .find(|e| e.0 == "WS")
        .expect("Cannot find WS URL in ENV")
        .1;
}

lazy_static! {
    pub static ref PROGRESS: Progress = Progress::read();
    pub static ref EVENT_CLIENT: EventClient = EventClient::default();
    pub static ref RELAY_CLIENT: Client = {
        let fb_signer = LocalWallet::new(&mut thread_rng());
        let signing_middleware = FlashbotsSignerLayer::new(fb_signer.clone());
        let service_builder = ServiceBuilder::new()
            .map_err(transport::Error::Http)
            .layer(signing_middleware);
        let rpc_client = HttpClientBuilder::default()
            .set_middleware(service_builder)
            .build(RELAY)
            .unwrap();
        Client {
            inner: Box::new(rpc_client),
        }
    };
    pub static ref RPC_CLIENT: Provider<ethers_providers::Http> =
        Provider::try_from(&*RPC_URL).expect("Could not connect to RPC endpoint");
    pub static ref WALLET: Wallet<SigningKey> = dotenv::vars()
        .find(|e| e.0 == "WALLET")
        .expect("Cannot find wallet private key in ENV")
        .1
        .parse::<LocalWallet>()
        .expect("Could not parse wallet private key")
        .with_chain_id(5u64);
}

lazy_static! {
    pub static ref LOGGER_CONTRACT: H160 =
        H160::from_str("0x6c9c151642c0ba512de540bd007afa70be2f1312").unwrap();
    pub static ref SIMPLE_CONTRACT_1: H160 =
        H160::from_str("0x1cddb0ba9265bb3098982238637c2872b7d12474").unwrap();
    pub static ref SIMPLE_CONTRACT_2: H160 =
        H160::from_str("0x65459dd36b03af9635c06bad1930db660b968278").unwrap();
    pub static ref SIMPLE_CONTRACT_3: H160 =
        H160::from_str("0x20a1a5857fdff817aa1bd8097027a841d4969aa5").unwrap();
    pub static ref SIMPLE_CONTRACT_4: H160 =
        H160::from_str("0x98997b55bb271e254bec8b85763480719dab0e53").unwrap();
    pub static ref SIMPLE_CONTRACT_TRIPLE: H160 =
        H160::from_str("0x1ea6fb65bab1f405f8bdb26d163e6984b9108478").unwrap();
    pub static ref MAGIC_CONTRACT_1: H160 =
        H160::from_str("0x118bcb654d9a7006437895b51b5cd4946bf6cdc2").unwrap();
    pub static ref MAGIC_CONTRACT_2: H160 =
        H160::from_str("0x9be957d1c1c1f86ba9a2e1215e9d9eefde615a56").unwrap();
    pub static ref MAGIC_CONTRACT_3: H160 =
        H160::from_str("0xe8b7475e2790409715af793f799f3cc80de6f071").unwrap();
    pub static ref NEW_CONTRACT_CONTRACT: H160 =
        H160::from_str("0x5ea0fea0164e5aa58f407debb344876b5ee10dea").unwrap();
    pub static ref NEW_CONTRACT_INIT_CODE_HASH: H256 = {
        H256(keccak256(Bytes::from_str("0x60a060405233608052436000556080516101166100266000396000606f01526101166000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c806396b81609146037578063b88a802f146051575b600080fd5b603f60005481565b60405190815260200160405180910390f35b60576059565b005b4360005414606657600080fd5b600080819055507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b031663720ecf456040518163ffffffff1660e01b8152600401600060405180830381600087803b15801560c757600080fd5b505af115801560da573d6000803e3d6000fd5b5050505056fea26469706673582212207a00db890eff47285ac0d9c9b8735727d476952aa87b45ee82fd6bb4f42c6fa764736f6c63430008130033").unwrap()))
    };
    pub static ref CONTRACTS: HashSet<H160> = HashSet::from_iter(vec![
        *SIMPLE_CONTRACT_1,
        *SIMPLE_CONTRACT_2,
        *SIMPLE_CONTRACT_3,
        *SIMPLE_CONTRACT_4,
        *SIMPLE_CONTRACT_TRIPLE,
        *MAGIC_CONTRACT_1,
        *MAGIC_CONTRACT_2,
        *MAGIC_CONTRACT_3,
    ]);
}
