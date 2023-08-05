use std::{error::Error, str::FromStr};

use ethers_core::types::{Bytes, Eip1559TransactionRequest, H160, H256, U256};
use ethers_providers::Middleware;
use ethers_signers::Signer;
use futures_util::Future;
use mev_share_rpc_api::{BundleItem, Inclusion, SendBundleRequest};

use crate::constants::{
    BUNDLE_BLOCK_WINDOW, GAS_LIMIT, GWEI, MAX_GAS_PRICE, MAX_PRIORITY_FEE, PROGRESS, RELAY_CLIENT,
    RPC_CLIENT, TIP, WALLET,
};

pub async fn backrun_simple(tx_to_backrun: H256, to: H160) {
    backrun_handler(tx_to_backrun, to, |nonce| async move {
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
        Ok(vec![BundleItem::Tx {
            tx: new_bytes,
            can_revert: false,
        }])
    })
    .await
}

async fn backrun_handler<
    O: Future<Output = Result<Vec<BundleItem>, Box<dyn Error + Send + Sync>>>,
    F: Fn(U256) -> O,
>(
    tx_to_backrun: H256,
    to: H160,
    items: F,
) {
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
        let mut bundle_body = vec![BundleItem::Hash {
            hash: tx_to_backrun,
        }];
        let nonce = RPC_CLIENT
            .get_transaction_count(WALLET.address(), None)
            .await?;
        bundle_body.append(&mut items(nonce).await?.to_vec());
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
    .await;

    if let Err(e) = process {
        println!("Error processing address {}: {:?}", to, e);
    }

    PROGRESS.set_is_processing(to, false).await;
}
