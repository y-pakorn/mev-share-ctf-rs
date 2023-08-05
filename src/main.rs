#![allow(unused_imports)]

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    str::FromStr,
};

use ethers_core::{
    abi::AbiDecode,
    k256::{ecdsa::SigningKey, Secp256k1},
    rand::thread_rng,
    types::{
        transaction::eip2718::TypedTransaction, Block, Bytes, Eip1559TransactionRequest,
        Transaction, TransactionRequest, H160, H256,
    },
    utils::{keccak256, rlp::RlpStream},
};
use ethers_providers::{Middleware, Provider, Ws};
use ethers_signers::{LocalWallet, Signer, Wallet};
use futures_util::StreamExt;
use hyper::server::conn::Http;
use jsonrpsee::http_client::{
    transport::{self, Error as HttpError, HttpBackend},
    HttpClient, HttpClientBuilder,
};
use lazy_static::lazy_static;
use mev_share_rpc_api::{
    BundleItem, FlashbotsSigner, FlashbotsSignerLayer, Inclusion, MevApiClient, SendBundleRequest,
    SimBundleOverrides, Validity,
};
use mev_share_sse::{EventClient, FunctionSelector};
use progress::Progress;
use rayon::prelude::*;
use serde::Deserialize;
use tokio::runtime::Runtime;
use tower::{util::MapErr, ServiceBuilder};

pub mod error;
pub mod progress;

const SSE: &str = "https://mev-share-goerli.flashbots.net";
const RELAY: &str = "https://relay-goerli.flashbots.net:443";

lazy_static! {
    static ref RPC_URL: String = dotenv::vars()
        .find(|e| e.0 == "RPC")
        .expect("Cannot find RPC URL in ENV")
        .1;
    static ref WS_URL: String = dotenv::vars()
        .find(|e| e.0 == "WS")
        .expect("Cannot find WS URL in ENV")
        .1;
}

lazy_static! {
    static ref PROGRESS: Progress = Progress::read();
    static ref EVENT_CLIENT: EventClient = EventClient::default();
    static ref RELAY_CLIENT: Client = {
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
    static ref RPC_CLIENT: Provider<ethers_providers::Http> =
        Provider::try_from(&*RPC_URL).expect("Could not connect to RPC endpoint");
    static ref WALLET: Wallet<SigningKey> = dotenv::vars()
        .find(|e| e.0 == "WALLET")
        .expect("Cannot find wallet private key in ENV")
        .1
        .parse::<LocalWallet>()
        .expect("Could not parse wallet private key")
        .with_chain_id(5u64);
}

lazy_static! {
    static ref LOGGER_CONTRACT: H160 =
        H160::from_str("0x6c9c151642c0ba512de540bd007afa70be2f1312").unwrap();
    static ref SIMPLE_CONTRACT_1: H160 =
        H160::from_str("0x1cddb0ba9265bb3098982238637c2872b7d12474").unwrap();
    static ref SIMPLE_CONTRACT_2: H160 =
        H160::from_str("0x65459dd36b03af9635c06bad1930db660b968278").unwrap();
    static ref SIMPLE_CONTRACT_3: H160 =
        H160::from_str("0x20a1a5857fdff817aa1bd8097027a841d4969aa5").unwrap();
    static ref SIMPLE_CONTRACT_4: H160 =
        H160::from_str("0x98997b55bb271e254bec8b85763480719dab0e53").unwrap();
    static ref CONTRACTS: HashSet<H160> = HashSet::from_iter(vec![
        *SIMPLE_CONTRACT_1,
        *SIMPLE_CONTRACT_2,
        *SIMPLE_CONTRACT_3,
        *SIMPLE_CONTRACT_4,
    ]);
}

const MAX_GAS_PRICE: u128 = 100;
const MAX_PRIORITY_FEE: u128 = 100;
const GAS_LIMIT: u128 = 500000;
const GWEI: u128 = 1000000000;
const TIP: u128 = 100 * GWEI;

struct Client {
    pub inner: Box<dyn MevApiClient + Sync + Send>,
}

impl AsRef<dyn MevApiClient + Sync + Send> for Client {
    fn as_ref(&self) -> &(dyn MevApiClient + Send + Sync + 'static) {
        self.inner.as_ref()
    }
}

async fn backrun_simple(tx_to_backrun: H256, to: H160) {
    if PROGRESS.get_progress_for_address(to).await {
        println!("Skipping address {}: Already processed", to);
        return;
    }

    if PROGRESS.get_is_processing(to).await {
        println!("Skipping address {}: Still processing", to);
        return;
    }

    println!(
        "Processing transaction to {:?} backrunning {:?}!",
        to, tx_to_backrun
    );
    PROGRESS.set_is_processing(to, true).await;

    let process = async {
        let nonce = RPC_CLIENT
            .get_transaction_count(WALLET.address(), None)
            .await?;

        let tx = Eip1559TransactionRequest::new()
            .from(WALLET.address())
            .to(to)
            .data(Bytes::from_str("0xb88a802f").unwrap())
            .chain_id(5)
            .nonce(nonce)
            .max_priority_fee_per_gas(MAX_PRIORITY_FEE * GWEI + TIP)
            .max_fee_per_gas(MAX_GAS_PRICE * GWEI + TIP)
            .gas(GAS_LIMIT);
        let signature = WALLET.sign_transaction(&tx.clone().into()).await?;
        let bytes = tx.rlp_signed(&signature);
        let new_bytes = Bytes::from_str(&format!(
            "0x02{}",
            bytes.to_string().split("0x").collect::<Vec<&str>>()[1]
        ))
        .unwrap();

        let bundle_body = vec![
            BundleItem::Hash {
                hash: tx_to_backrun,
            },
            BundleItem::Tx {
                tx: new_bytes,
                can_revert: false,
            },
        ];
        let block = PROGRESS.get_latest_block().await;
        let bundle = SendBundleRequest {
            bundle_body,
            inclusion: Inclusion {
                block,
                max_block: Some(block + 5),
            },
            ..Default::default()
        };

        let resp = RELAY_CLIENT.as_ref().send_bundle(bundle.clone()).await?;
        println!("Got a bundle response: {:?}", resp);

        Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
    }
    .await;

    match process {
        Ok(_) => {
            //println!("Finished processing address {}", to);
            //PROGRESS.add_progress_for_address(to).await;
        }
        Err(e) => {
            println!("Error processing address {}: {:?}", to, e);
        }
    }

    PROGRESS.set_is_processing(to, false).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Interacting with wallet: {:?}", WALLET.address());

    let f1 = async {
        let client = Provider::connect(&*WS_URL)
            .await
            .expect("Could not connect to WS endpoint");
        let mut stream = client.subscribe_blocks().await?;
        while let Some(Block {
            hash: Some(hash),
            number: Some(number),
            timestamp,
            ..
        }) = stream.next().await
        {
            println!("Got block {}: {:?} at {:?}", number, hash, timestamp);
            PROGRESS.set_latest_block(number).await;

            if let Some(block) = RPC_CLIENT.get_block_with_txs(hash).await? {
                block.transactions.iter().for_each(|tx| {
                    if let Some(to) = &tx.to {
                        let tx = tx.clone();
                        let to = *to;
                        tokio::spawn(async move {
                            if CONTRACTS.contains(&to) && tx.from == WALLET.address() {
                                println!("Found tx sent: {:?}", tx.hash);
                                PROGRESS.add_progress_for_address(to).await;
                            }
                        });
                    }
                });
            }
        }
        Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
    };
    let f2 = async {
        let mut stream = EVENT_CLIENT.events(SSE).await?;
        println!("Subscribed to {}", stream.endpoint());

        while let Some(Ok(event)) = stream.next().await {
            event.transactions.iter().for_each(|tx| {
                let tx = tx.clone();
                tokio::spawn(async move {
                    if tx.to == Some(*SIMPLE_CONTRACT_1)
                        && tx
                            .function_selector
                            .as_ref()
                            .map(|e| e.to_string() == "0xa3c356e4")
                            .unwrap_or_default()
                        && tx
                            .calldata
                            .as_ref()
                            .map(|e| e.to_string() == "0xa3c356e4")
                            .unwrap_or_default()
                    {
                        backrun_simple(event.hash, tx.to.unwrap()).await;
                    }

                    if tx.to == Some(*SIMPLE_CONTRACT_2)
                        && tx
                            .function_selector
                            .as_ref()
                            .map(|e| e.to_string() == "0xa3c356e4")
                            .unwrap_or_default()
                        && tx.calldata.is_none()
                    {
                        backrun_simple(event.hash, tx.to.unwrap()).await;
                    }

                    if tx.to.is_none() && tx.function_selector.is_none() && tx.calldata.is_none() {
                        backrun_simple(event.hash, tx.to.unwrap()).await;
                    }
                });
            });

            event.logs.iter().for_each(|log| {
            let log = log.clone();
            tokio::spawn(async move {
                //println!("{}", &event.hash);
                //dbg!(&log);

                if log.address == *SIMPLE_CONTRACT_4
                    && log.topics.get(0).map(|t| *t == H256::from_str( "0x59d3ce47d6ad6c6003cef97d136155b29d88653eb355c8bed6e03fbf694570ca").unwrap()).unwrap_or_default()
                {
                    backrun_simple(event.hash, log.address).await;
                }
            });
        });
        }

        Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
    };
    let (r1, r2) = tokio::join!(f1, f2);

    r1.expect("WebSocket failed");
    r2.expect("Event stream failed");

    Ok(())
}
