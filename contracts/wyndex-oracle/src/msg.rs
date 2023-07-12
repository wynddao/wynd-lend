use crate::state::Config;
use utils::wyndex::{SimulateSwapOperationsResponse, SwapOperation};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use wyndex::{
    asset::AssetInfo,
    oracle::{SamplePeriod, TwapResponse},
};

#[cw_serde]
pub struct InstantiateMsg {
    pub controller: String,
    /// Multi-hop address of wynddex factory that allows to swap and query price
    /// using multiple routes
    pub multi_hop: String,
    /// TWAP parameters
    /// Number of full sample periods to average.
    /// 4 would start 4 full sample periods earlier than the end of the time buffer
    pub start_age: u32,
    /// Resolution of the buffer we wish to read
    pub sample_period: SamplePeriod,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Register an Wynddex liquidity pool for a given trading pair. Only callable by the controller.
    /// The order of denoms doesn't matter.
    RegisterPool {
        pair_contract: String,
        denom1: AssetInfo,
        denom2: AssetInfo,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the oracle configuration.
    #[returns(Config)]
    Config {},
    /// Simulate swap operations on wynddex's multi hop contract.
    #[returns(SimulateSwapOperationsResponse)]
    SimulateSwapOperations {
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
    #[returns(SimulateSwapOperationsResponse)]
    SimulateReverseSwapOperations {
        ask_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
    #[returns(TwapResponse)]
    Twap { offer: AssetInfo, ask: AssetInfo },
    /// Returs configured liquidity pool's address for a given pair of assets
    #[returns(cosmwasm_std::Addr)]
    PoolAddress {
        first_asset: AssetInfo,
        second_asset: AssetInfo,
    },
}

#[cw_serde]
pub struct PriceResponse {
    pub rate: Decimal,
}
