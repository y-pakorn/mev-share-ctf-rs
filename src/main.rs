use std::error::Error;

use ethers_core::types::Block;
use ethers_providers::{Middleware, Provider};
use ethers_signers::Signer;
use futures_util::StreamExt;
use mev_share_sse::Event;

use crate::{
    constants::{CONTRACTS, EVENT_CLIENT, PROGRESS, RPC_CLIENT, SSE, WALLET, WS_URL},
    executor::Executor,
};

pub mod client;
pub mod constants;
pub mod contracts;
pub mod error;
pub mod executor;
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
            Executor::execute_event(&event);

            let Event {
                hash,
                transactions,
                logs,
            } = event;

            for tx in transactions {
                tokio::spawn(async move {
                    Executor::execute_tx(hash, &tx);
                });
            }

            for log in logs {
                tokio::spawn(async move {
                    Executor::execute_log(hash, &log);
                });
            }
        }

        Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
    };
    let (r1, r2) = tokio::join!(f1, f2);

    r1.expect("WebSocket failed");
    r2.expect("Event stream failed");

    Ok(())
}
