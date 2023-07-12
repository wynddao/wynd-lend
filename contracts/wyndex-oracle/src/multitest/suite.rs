use anyhow::Result as AnyResult;

use cosmwasm_std::{Addr, Coin, Decimal, StdResult, Uint128};

use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};

use wyndex::{
    asset::AssetInfo,
    oracle::{SamplePeriod, TwapResponse},
};
use wyndex_tests::builder::{WyndexSuite, WyndexSuiteBuilder};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::TWAPParams,
};
use utils::wyndex::{SimulateSwapOperationsResponse, SwapOperation};

fn store_oracle(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    ));

    app.store_code(contract)
}

#[derive(Debug, Eq, PartialEq)]
pub struct SimulateSwapResponse {
    pub amount: Uint128,
    pub spread: Decimal,
}

#[derive(Debug)]
pub struct SuiteBuilder {
    funds: Vec<(Addr, Vec<Coin>)>,
    controller: String,
    twap_params: TWAPParams,
}

#[allow(dead_code)]
impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            funds: vec![],
            controller: "admin".to_owned(),
            twap_params: TWAPParams {
                start_age: 1,
                sample_period: SamplePeriod::HalfHour,
            },
        }
    }

    pub fn with_funds(mut self, addr: &str, funds: &[Coin]) -> Self {
        self.funds.push((Addr::unchecked(addr), funds.into()));
        self
    }

    pub fn with_controller(mut self, name: &str) -> Self {
        self.controller = name.to_owned();
        self
    }

    pub fn with_twap_params(mut self, start_age: u32, sample_period: SamplePeriod) -> Self {
        self.twap_params = TWAPParams {
            start_age,
            sample_period,
        };
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app = App::default();
        let owner = Addr::unchecked("owner");

        // initialize wyndex test dependencies.
        let wyndex_builder = WyndexSuiteBuilder {
            owner: owner.clone(),
        };
        let wyndex_suite = wyndex_builder.init_wyndex(&mut app);

        let oracle_id = store_oracle(&mut app);
        let oracle = app
            .instantiate_contract(
                oracle_id,
                owner.clone(),
                &InstantiateMsg {
                    controller: self.controller.clone(),
                    multi_hop: wyndex_suite.multi_hop.address.to_string(),
                    start_age: self.twap_params.start_age,
                    sample_period: self.twap_params.sample_period,
                },
                &[],
                "Wyndex Oracle",
                None,
            )
            .unwrap();

        let funds = self.funds;
        app.init_modules(|router, _, storage| -> AnyResult<()> {
            for (addr, coin) in funds {
                router.bank.init_balance(storage, &addr, coin)?;
            }
            Ok(())
        })
        .unwrap();

        Suite {
            owner: owner.to_string(),
            app,
            wyndex: wyndex_suite,
            oracle,
        }
    }
}

pub struct Suite {
    pub owner: String,
    pub app: App,
    pub wyndex: WyndexSuite,
    pub oracle: Addr,
}

impl Suite {
    pub fn advance_seconds(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.time = block.time.plus_seconds(seconds);
            block.height += std::cmp::max(1, seconds / 5); // block time
        });
    }

    pub fn register_pool(
        &mut self,
        sender: &str,
        pair_address: &str,
        denom1: &AssetInfo,
        denom2: &AssetInfo,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            Addr::unchecked(self.oracle.clone()),
            &ExecuteMsg::RegisterPool {
                pair_contract: pair_address.to_owned(),
                denom1: denom1.clone(),
                denom2: denom2.clone(),
            },
            &[],
        )
    }

    pub fn query_pool_address(
        &self,
        first_asset: &AssetInfo,
        second_asset: &AssetInfo,
    ) -> StdResult<Addr> {
        self.app.wrap().query_wasm_smart::<Addr>(
            self.oracle.clone(),
            &QueryMsg::PoolAddress {
                first_asset: first_asset.clone(),
                second_asset: second_asset.clone(),
            },
        )
    }

    pub fn query_twap(
        &self,
        first_asset: &AssetInfo,
        second_asset: &AssetInfo,
    ) -> StdResult<TwapResponse> {
        self.app.wrap().query_wasm_smart::<TwapResponse>(
            self.oracle.clone(),
            &QueryMsg::Twap {
                offer: first_asset.clone(),
                ask: second_asset.clone(),
            },
        )
    }

    pub fn simulate_swap_operations(
        &self,
        offer_amount: impl Into<Uint128>,
        operations: Vec<SwapOperation>,
    ) -> StdResult<SimulateSwapResponse> {
        let response = self
            .app
            .wrap()
            .query_wasm_smart::<SimulateSwapOperationsResponse>(
                self.oracle.clone(),
                &QueryMsg::SimulateSwapOperations {
                    offer_amount: offer_amount.into(),
                    operations,
                },
            )
            .unwrap();
        Ok(SimulateSwapResponse {
            amount: response.amount,
            spread: response.spread,
        })
    }

    pub fn simulate_reverse_swap_operations(
        &self,
        ask_amount: impl Into<Uint128>,
        operations: Vec<SwapOperation>,
    ) -> StdResult<SimulateSwapResponse> {
        let response = self
            .app
            .wrap()
            .query_wasm_smart::<SimulateSwapOperationsResponse>(
                self.oracle.clone(),
                &QueryMsg::SimulateReverseSwapOperations {
                    ask_amount: ask_amount.into(),
                    operations,
                },
            )
            .unwrap();
        Ok(SimulateSwapResponse {
            amount: response.amount,
            spread: response.spread,
        })
    }
}
