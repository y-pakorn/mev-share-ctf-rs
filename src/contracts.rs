use ethers_core::abi::{parse_abi, Abi};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref MAGIC_NUMBER_ABI: Abi = parse_abi(&[
        "event Activate(uint256 lowerBound, uint256 upperBound)",
        "function claimReward(uint256 _magicNumber)",
    ])
    .unwrap();
    pub static ref NEW_CONTRACT_ABI: Abi = parse_abi(&[
        "event Activate(address newlyDeployedContract)",
        "event ActivateBySalt(bytes32 salt)",
        "function claimReward()"
    ])
    .unwrap();
}
