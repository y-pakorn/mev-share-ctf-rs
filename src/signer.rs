use std::{error::Error, str::FromStr};

use ethers_core::types::{Bytes, Eip1559TransactionRequest};
use ethers_signers::Signer;

use crate::constants::{GAS_LIMIT, GWEI, MAX_GAS_PRICE, MAX_PRIORITY_FEE, TIP, WALLET};

pub async fn sign_transaction(
    tx: Eip1559TransactionRequest,
) -> Result<Bytes, Box<dyn Error + Sync + Send>> {
    let tx = tx
        .from(WALLET.address())
        .chain_id(5)
        .max_priority_fee_per_gas(MAX_PRIORITY_FEE * GWEI + TIP)
        .max_fee_per_gas(MAX_GAS_PRICE * GWEI + TIP)
        .gas(GAS_LIMIT);
    let signature = WALLET.sign_transaction(&tx.clone().into()).await?;
    let bytes = tx.rlp_signed(&signature);
    Ok(Bytes::from_str(&format!(
        "0x02{}",
        bytes.to_string().split("0x").collect::<Vec<&str>>()[1]
    ))
    .unwrap())
}
