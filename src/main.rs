use std::{error::Error, str::FromStr};

use ethers_core::types::{Block, H256};
use ethers_providers::{Middleware, Provider};
use ethers_signers::Signer;
use futures_util::StreamExt;

use crate::{
    constants::{
        CONTRACTS, EVENT_CLIENT, MAGIC_CONTRACT_1, MAGIC_CONTRACT_2, MAGIC_CONTRACT_3, PROGRESS,
        RPC_CLIENT, SIMPLE_CONTRACT_1, SIMPLE_CONTRACT_2, SIMPLE_CONTRACT_3, SIMPLE_CONTRACT_4,
        SIMPLE_CONTRACT_TRIPLE, SSE, WALLET, WS_URL,
    },
    handler::{backrun_magic_numba, backrun_simple, backrun_simple_triple},
};

pub mod client;
pub mod constants;
pub mod contracts;
pub mod error;
pub mod handler;
pub mod progress;
pub mod signer;

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
            if event.logs.is_empty() && event.transactions.is_empty() {
                backrun_simple(event.hash, *SIMPLE_CONTRACT_3).await
            }

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
                        backrun_simple(event.hash, tx.to.unwrap()).await
                    }

                    if tx.to == Some(*SIMPLE_CONTRACT_2)
                        && tx
                            .function_selector
                            .as_ref()
                            .map(|e| e.to_string() == "0xa3c356e4")
                            .unwrap_or_default()
                        && tx.calldata.is_none()
                    {
                        backrun_simple(event.hash, tx.to.unwrap()).await
                    }
                });
            });

            event.logs.iter().for_each(|log| {
                let log = log.clone();
                let event = event.clone();
                tokio::spawn(async move {
                    if log.address == *SIMPLE_CONTRACT_4
                        && log.topics.get(0).map(|t| *t == H256::from_str("0x59d3ce47d6ad6c6003cef97d136155b29d88653eb355c8bed6e03fbf694570ca").unwrap()).unwrap_or_default()
                    {
                        backrun_simple(event.hash, log.address).await
                    }

                    if log.address == *SIMPLE_CONTRACT_TRIPLE
                        && log.topics.get(0).map(|t| *t == H256::from_str("0x59d3ce47d6ad6c6003cef97d136155b29d88653eb355c8bed6e03fbf694570ca").unwrap()).unwrap_or_default()
                    {
                        backrun_simple_triple(event.hash, log.address).await
                    }

                    if log.address == *MAGIC_CONTRACT_1 && log.topics.get(0).map(|t| *t == H256::from_str("0x86a27c2047f889fafe51029e28e24f466422abe8a82c0c27de4683dda79a0b5d").unwrap()).unwrap_or_default() {
                        backrun_magic_numba(event.hash, log.address, &log.data).await
                    }

                    if log.address == *MAGIC_CONTRACT_2 && log.topics.get(0).map(|t| *t == H256::from_str("0x86a27c2047f889fafe51029e28e24f466422abe8a82c0c27de4683dda79a0b5d").unwrap()).unwrap_or_default() {
                        backrun_magic_numba(event.hash, log.address, &log.data).await
                    }

                    if log.address == *MAGIC_CONTRACT_3 && log.topics.get(0).map(|t| *t == H256::from_str("0x86a27c2047f889fafe51029e28e24f466422abe8a82c0c27de4683dda79a0b5d").unwrap()).unwrap_or_default() {
                        backrun_magic_numba(event.hash, log.address, &log.data).await
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
