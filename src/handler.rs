use std::{error::Error, str::FromStr};

use ethers_core::{
    abi::{RawLog, Token},
    types::{Bytes, Eip1559TransactionRequest, H160, H256, U256},
};
use ethers_providers::Middleware;
use ethers_signers::Signer;
use futures_util::Future;
use mev_share_rpc_api::{BundleItem, Inclusion, SendBundleRequest};

use crate::{
    constants::{BUNDLE_BLOCK_WINDOW, PROGRESS, RELAY_CLIENT, RPC_CLIENT, WALLET},
    contracts::MAGIC_NUMBER_ABI,
    signer::sign_transaction,
};

pub async fn backrun_magic_numba(tx_to_backrun: H256, to: H160, bound_data: &Bytes) {
    if let Err(err) = async {
        let nonce = RPC_CLIENT
            .get_transaction_count(WALLET.address(), None)
            .await?;

        let mut bounds = MAGIC_NUMBER_ABI
            .event("Activate")?
            .parse_log(RawLog {
                topics: vec![H256::from_str(
                    "0x86a27c2047f889fafe51029e28e24f466422abe8a82c0c27de4683dda79a0b5d",
                )?],
                data: bound_data.to_vec(),
            })?
            .params
            .into_iter();
        let lower_b = bounds.next().unwrap().value.into_uint().unwrap();
        let upper_b = bounds.next().unwrap().value.into_uint().unwrap();

        let mut magic_number = lower_b;
        while magic_number <= upper_b {
            magic_number += U256::one();
            tokio::spawn(async move {
                backrun_handler(tx_to_backrun, to, async move {
                    let tx_body = Bytes::from(
                        MAGIC_NUMBER_ABI
                            .function("claimReward")?
                            .encode_input(&[Token::Uint(magic_number)])?,
                    );
                    let tx = Eip1559TransactionRequest::new()
                        .to(to)
                        .data(tx_body)
                        .nonce(nonce);
                    let bytes = sign_transaction(tx).await?;
                    Ok(vec![BundleItem::Tx {
                        tx: bytes,
                        can_revert: false,
                    }])
                })
                .await;
            });
        }

        Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
    }
    .await
    {
        println!("Error getting nonce: {:?}", err);
    }
}

pub async fn backrun_simple(tx_to_backrun: H256, to: H160) {
    backrun_handler(tx_to_backrun, to, async move {
        let nonce = RPC_CLIENT
            .get_transaction_count(WALLET.address(), None)
            .await?;
        let tx = Eip1559TransactionRequest::new()
            .to(to)
            .data(Bytes::from_str("0xb88a802f")?)
            .nonce(nonce);
        let bytes = sign_transaction(tx).await?;
        Ok(vec![BundleItem::Tx {
            tx: bytes,
            can_revert: false,
        }])
    })
    .await
}

async fn backrun_handler<
    O: Future<Output = Result<Vec<BundleItem>, Box<dyn Error + Send + Sync>>>,
>(
    tx_to_backrun: H256,
    to: H160,
    items: O,
) {
    if PROGRESS.get_progress_for_address(to).await {
        //println!("Skipping address {}: Already processed", to);
        return;
    }

    println!(
        "Processing transaction to {:?} backrunning {:?}!",
        to, tx_to_backrun
    );
    PROGRESS.set_is_processing(to, true).await;

    if let Err(e) = async {
        let mut bundle_body = vec![BundleItem::Hash {
            hash: tx_to_backrun,
        }];
        bundle_body.append(&mut items.await?.to_vec());
        let block = PROGRESS.get_latest_block().await;
        let bundle = SendBundleRequest {
            bundle_body,
            inclusion: Inclusion {
                block,
                max_block: Some(block + BUNDLE_BLOCK_WINDOW),
            },
            ..Default::default()
        };

        let resp = RELAY_CLIENT.as_ref().send_bundle(bundle.clone()).await?;
        println!("Got a bundle response: {:?}", resp);

        Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
    }
    .await
    {
        println!("Error processing address {}: {:?}", to, e);
    }
}
