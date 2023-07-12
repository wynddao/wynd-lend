#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;

use wyndex::{
    asset::AssetInfo,
    oracle::{SamplePeriod, TwapResponse},
    pair::QueryMsg as PairQueryMsg,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use crate::utils::sorted_tuple;
use crate::{error::ContractError, state::TWAPParams};
use utils::wyndex::SimulateSwapOperationsResponse;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:wyndex-oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // TODO: should we add some check on start age?
    let cfg = Config {
        controller: deps.api.addr_validate(&msg.controller)?,
        multi_hop: deps.api.addr_validate(&msg.multi_hop)?,
        twap_params: TWAPParams {
            start_age: msg.start_age,
            sample_period: msg.sample_period,
        },
    };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;
    match msg {
        RegisterPool {
            pair_contract,
            denom1,
            denom2,
        } => execute::register_pool(deps, info, pair_contract, denom1, denom2),
    }
}

mod execute {
    use cosmwasm_std::ensure_eq;

    use crate::state::POOLS;

    use super::*;

    pub fn register_pool(
        deps: DepsMut,
        info: MessageInfo,
        pair_contract: String,
        denom1: AssetInfo,
        denom2: AssetInfo,
    ) -> Result<Response, ContractError> {
        let cfg = CONFIG.load(deps.storage)?;
        ensure_eq!(info.sender, cfg.controller, ContractError::Unauthorized {});

        let pair_address = deps.api.addr_validate(&pair_contract)?;
        POOLS.save(
            deps.storage,
            sorted_tuple(denom1.as_bytes(), denom2.as_bytes()),
            &pair_address,
        )?;

        Ok(Response::new().add_attribute("action", "register_pool"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;
    match msg {
        Config {} => to_binary(&query::config(deps)?),
        SimulateSwapOperations {
            offer_amount,
            operations,
        } => to_binary(&query::simulate_swap_operations(
            deps,
            offer_amount,
            operations,
        )?),
        SimulateReverseSwapOperations {
            ask_amount,
            operations,
        } => to_binary(&query::simulate_reverse_swap_operations(
            deps, ask_amount, operations,
        )?),
        Twap { offer, ask } => {
            let cfg = CONFIG.load(deps.storage)?;
            to_binary(&query::twap(
                deps,
                offer,
                ask,
                cfg.twap_params.start_age,
                cfg.twap_params.sample_period,
            )?)
        }
        PoolAddress {
            first_asset,
            second_asset,
        } => to_binary(&query::pool_address(deps, &first_asset, &second_asset)?),
    }
}

mod query {
    use cosmwasm_std::{Addr, QueryRequest, WasmQuery};

    use crate::state::POOLS;
    use utils::wyndex::{MultiHopQueryMsg, SwapOperation};

    use super::*;

    pub fn config(deps: Deps) -> StdResult<Config> {
        CONFIG.load(deps.storage)
    }

    pub fn simulate_swap_operations(
        deps: Deps,
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    ) -> StdResult<SimulateSwapOperationsResponse> {
        let multi_hop = CONFIG.load(deps.storage)?.multi_hop;
        deps.querier
            .query::<SimulateSwapOperationsResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: multi_hop.to_string(),
                msg: to_binary(&MultiHopQueryMsg::SimulateSwapOperations {
                    offer_amount,
                    operations,
                    referral: false,
                    referral_commission: None,
                })?,
            }))
    }

    pub fn simulate_reverse_swap_operations(
        deps: Deps,
        ask_amount: Uint128,
        operations: Vec<SwapOperation>,
    ) -> StdResult<SimulateSwapOperationsResponse> {
        let multi_hop = CONFIG.load(deps.storage)?.multi_hop;
        deps.querier
            .query::<SimulateSwapOperationsResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: multi_hop.to_string(),
                msg: to_binary(&MultiHopQueryMsg::SimulateReverseSwapOperations {
                    ask_amount,
                    operations,
                    referral: false,
                    referral_commission: None,
                })?,
            }))
    }

    pub fn twap(
        deps: Deps,
        offer: AssetInfo,
        ask: AssetInfo,
        start_age: u32,
        duration: SamplePeriod,
    ) -> StdResult<TwapResponse> {
        let pool = pool_address(deps, &offer, &ask)?.to_string();
        deps.querier
            .query::<TwapResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pool.to_string(),
                msg: to_binary(&PairQueryMsg::Twap {
                    duration,
                    start_age,
                    end_age: Some(0),
                })?,
            }))
    }

    pub fn pool_address(deps: Deps, denom1: &AssetInfo, denom2: &AssetInfo) -> StdResult<Addr> {
        POOLS
            .may_load(
                deps.storage,
                sorted_tuple(denom1.as_bytes(), denom2.as_bytes()),
            )?
            .ok_or_else(|| StdError::GenericErr {
                msg: format!(
                    "There is no info about the contract address of pair {} and {}",
                    denom1, denom2
                ),
            })
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };

    use super::*;

    #[test]
    fn config() {
        let mut deps = mock_dependencies();
        let instantiate_msg = InstantiateMsg {
            controller: "control".to_owned(),
            multi_hop: "multi-hop".to_owned(),
            start_age: 4,
            sample_period: SamplePeriod::HalfHour,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

        let config: Config =
            from_binary(&query(deps.as_ref(), env, QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            Config {
                controller: Addr::unchecked("control"),
                multi_hop: Addr::unchecked("multi-hop"),
                twap_params: TWAPParams {
                    start_age: 4,
                    sample_period: SamplePeriod::HalfHour,
                }
            }
        );
    }
}
