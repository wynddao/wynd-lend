use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};
use wyndex::oracle::SamplePeriod;

#[cw_serde]
pub struct Config {
    pub controller: Addr,
    pub multi_hop: Addr,
    pub twap_params: TWAPParams,
}

#[cw_serde]
pub struct PriceRecord {
    pub rate: Decimal,
    // pub expires: Expiration,
}

#[cw_serde]
pub struct TWAPParams {
    pub start_age: u32,
    pub sample_period: SamplePeriod,
}

pub const CONFIG: Item<Config> = Item::new("config");
/// The list of all pools the oracle is aware of. The denoms are expected to be given in ascending order
pub const POOLS: Map<(&[u8], &[u8]), Addr> = Map::new("liquidity_pools");
