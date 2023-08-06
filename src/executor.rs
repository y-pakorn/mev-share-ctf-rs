use std::str::FromStr;

use ethers_core::types::H256;
use futures_util::future::BoxFuture;
use mev_share_sse::{Event, EventTransaction, EventTransactionLog};

use crate::{
    constants::{
        MAGIC_CONTRACT_1, MAGIC_CONTRACT_2, MAGIC_CONTRACT_3, NEW_CONTRACT_CONTRACT,
        SIMPLE_CONTRACT_1, SIMPLE_CONTRACT_2, SIMPLE_CONTRACT_3, SIMPLE_CONTRACT_4,
        SIMPLE_CONTRACT_TRIPLE,
    },
    handler::{
        backrun_create_contract_addr, backrun_create_contract_salt, backrun_magic_numba,
        backrun_simple, backrun_simple_triple,
    },
};

pub struct Executor;

pub type Predicate<T> = fn(&T) -> bool;
pub type Handler<T> = fn(T) -> BoxFuture<'static, ()>;
pub type HashHandler<T> = fn(H256, T) -> BoxFuture<'static, ()>;

impl Executor {
    pub fn handle_event() -> Vec<(Predicate<Event>, Handler<Event>)> {
        vec![(
            |event| event.logs.is_empty() && event.transactions.is_empty(),
            |event| Box::pin(backrun_simple(event.hash, *SIMPLE_CONTRACT_3)),
        )]
    }

    pub fn handle_tx() -> Vec<(Predicate<EventTransaction>, HashHandler<EventTransaction>)> {
        vec![
            (
                |tx| {
                    tx.to == Some(*SIMPLE_CONTRACT_1)
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
                },
                |hash, tx| Box::pin(backrun_simple(hash, tx.to.unwrap())),
            ),
            (
                |tx| {
                    tx.to == Some(*SIMPLE_CONTRACT_2)
                        && tx
                            .function_selector
                            .as_ref()
                            .map(|e| e.to_string() == "0xa3c356e4")
                            .unwrap_or_default()
                        && tx.calldata.is_none()
                },
                |hash, tx| Box::pin(backrun_simple(hash, tx.to.unwrap())),
            ),
        ]
    }

    pub fn handle_log() -> Vec<(
        Predicate<EventTransactionLog>,
        HashHandler<EventTransactionLog>,
    )> {
        vec![
            (
                |log| {
                    log.address == *SIMPLE_CONTRACT_4
                        && log.topics.get(0).map(|t| *t == H256::from_str("0x59d3ce47d6ad6c6003cef97d136155b29d88653eb355c8bed6e03fbf694570ca").unwrap()).unwrap_or_default()
                },
                |hash, log| Box::pin(backrun_simple(hash, log.address)),
            ),
            (
                |log| {
                    log.address == *SIMPLE_CONTRACT_TRIPLE
                        && log.topics.get(0).map(|t| *t == H256::from_str("0x59d3ce47d6ad6c6003cef97d136155b29d88653eb355c8bed6e03fbf694570ca").unwrap()).unwrap_or_default()
                },
                |hash, log| Box::pin(backrun_simple_triple(hash, log.address)),
            ),
            (
                |log| {
                    log.address == *MAGIC_CONTRACT_1 && log.topics.get(0).map(|t| *t == H256::from_str("0x86a27c2047f889fafe51029e28e24f466422abe8a82c0c27de4683dda79a0b5d").unwrap()).unwrap_or_default()
                },
                |hash, log| {
                    Box::pin(async move { backrun_magic_numba(hash, log.address, &log.data).await })
                },
            ),
            (
                |log| {
                    log.address == *MAGIC_CONTRACT_2 && log.topics.get(0).map(|t| *t == H256::from_str("0x86a27c2047f889fafe51029e28e24f466422abe8a82c0c27de4683dda79a0b5d").unwrap()).unwrap_or_default()
                },
                |hash, log| {
                    Box::pin(async move { backrun_magic_numba(hash, log.address, &log.data).await })
                },
            ),
            (
                |log| {
                    log.address == *MAGIC_CONTRACT_3 && log.topics.get(0).map(|t| *t == H256::from_str("0x86a27c2047f889fafe51029e28e24f466422abe8a82c0c27de4683dda79a0b5d").unwrap()).unwrap_or_default()
                },
                |hash, log| {
                    Box::pin(async move { backrun_magic_numba(hash, log.address, &log.data).await })
                },
            ),
            (
                |log| {
                    log.address == *NEW_CONTRACT_CONTRACT && log.topics.get(0).map(|t| *t == H256::from_str("0xf7e9fe69e1d05372bc855b295bc4c34a1a0a5882164dd2b26df30a26c1c8ba15").unwrap()).unwrap_or_default()
                },
                |hash, log| {
                    Box::pin(async move {
                        backrun_create_contract_addr(hash, log.address, &log.data).await
                    })
                },
            ),
            (
                |log| {
                    log.address == *NEW_CONTRACT_CONTRACT && log.topics.get(0).map(|t| *t == H256::from_str("0x71fd33d3d871c60dc3d6ecf7c8e5bb086aeb6491528cce181c289a411582ff1c").unwrap()).unwrap_or_default()
                },
                |hash, log| {
                    Box::pin(async move {
                        backrun_create_contract_salt(hash, log.address, &log.data).await
                    })
                },
            ),
        ]
    }

    pub fn execute_event(event: &Event) {
        for (predicate, handler) in Self::handle_event() {
            let event = event.clone();
            tokio::spawn(async move {
                if predicate(&event) {
                    handler(event).await;
                }
            });
        }
    }

    pub fn execute_tx(hash: H256, tx: &EventTransaction) {
        for (predicate, handler) in Self::handle_tx() {
            let tx = tx.clone();
            tokio::spawn(async move {
                if predicate(&tx) {
                    handler(hash, tx).await;
                }
            });
        }
    }

    pub fn execute_log(hash: H256, log: &EventTransactionLog) {
        for (predicate, handler) in Self::handle_log() {
            let log = log.clone();
            tokio::spawn(async move {
                if predicate(&log) {
                    handler(hash, log).await;
                }
            });
        }
    }
}
