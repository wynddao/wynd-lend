use anyhow::Result as AnyResult;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, to_binary, Addr, Coin, Decimal, Empty, Uint128};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20BaseInstantiateMsg;
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use wyndex::{
    asset::{Asset, AssetInfo},
    factory::{
        DefaultStakeConfig, ExecuteMsg as FactoryExecuteMsg,
        InstantiateMsg as FactoryInstantiateMsg, PairConfig, PairType, PartialStakeConfig,
        QueryMsg as FactoryQueryMsg,
    },
    fee_config::FeeConfig,
    pair::{Cw20HookMsg, ExecuteMsg as PairExecuteMsg, PairInfo, StablePoolParams},
};

use wyndex_multi_hop::msg::InstantiateMsg as MultiHopInstantiateMsg;

const SECONDS_PER_DAY: u64 = 60 * 60 * 24;

// -------------------------------------------------------------------------------------------------
// Contracts
// -------------------------------------------------------------------------------------------------

fn contract_mock_hub() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::mock_hub::execute,
        crate::mock_hub::instantiate,
        crate::mock_hub::query,
    );

    Box::new(contract)
}

/// Contract code that manages LSD pairs.
fn contract_pair_lsd() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        wyndex_pair_lsd::contract::execute,
        wyndex_pair_lsd::contract::instantiate,
        wyndex_pair_lsd::contract::query,
    )
    .with_reply_empty(wyndex_pair_lsd::contract::reply);

    Box::new(contract)
}

/// Contract code that manages standard pairs.
fn contract_pair() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        wyndex_pair::contract::execute,
        wyndex_pair::contract::instantiate,
        wyndex_pair::contract::query,
    )
    .with_reply_empty(wyndex_pair::contract::reply);

    Box::new(contract)
}

/// Contract code that manages cw20 tokens.
fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );

    Box::new(contract)
}

fn contract_staking() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        wyndex_stake::contract::execute,
        wyndex_stake::contract::instantiate,
        wyndex_stake::contract::query,
    );

    Box::new(contract)
}

fn contract_factory() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        wyndex_factory::contract::execute,
        wyndex_factory::contract::instantiate,
        wyndex_factory::contract::query,
    )
    .with_reply_empty(wyndex_factory::contract::reply);

    Box::new(contract)
}

fn contract_multi_hop() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        wyndex_multi_hop::contract::execute,
        wyndex_multi_hop::contract::instantiate,
        wyndex_multi_hop::contract::query,
    );

    Box::new(contract)
}

// -------------------------------------------------------------------------------------------------
// Init
// -------------------------------------------------------------------------------------------------
fn init_mock_hub(app: &mut App, mock_hub_code_id: u64, owner: Addr) -> Addr {
    app.instantiate_contract(
        mock_hub_code_id,
        owner,
        &Decimal::one(),
        &[],
        "Mock Hub",
        None,
    )
    .unwrap()
}

// Utility function to initialize the wyndex factory smart contract
fn init_factory(
    app: &mut App,
    factory_code_id: u64,
    owner: Addr,
    pair_lsd_code_id: u64,
    pair_code_id: u64,
    cw20_code_id: u64,
    staking_code_id: u64,
) -> Addr {
    app.instantiate_contract(
        factory_code_id,
        owner.clone(),
        &FactoryInstantiateMsg {
            pair_configs: vec![
                PairConfig {
                    code_id: pair_lsd_code_id,
                    pair_type: PairType::Lsd {},
                    fee_config: FeeConfig {
                        total_fee_bps: 0,
                        protocol_fee_bps: 0,
                    },
                    is_disabled: false,
                },
                PairConfig {
                    code_id: pair_code_id,
                    pair_type: PairType::Xyk {},
                    fee_config: FeeConfig {
                        total_fee_bps: 0,
                        protocol_fee_bps: 0,
                    },
                    is_disabled: false,
                },
            ],
            token_code_id: cw20_code_id,
            fee_address: None,
            owner: owner.to_string(),
            max_referral_commission: Decimal::one(),
            default_stake_config: DefaultStakeConfig {
                staking_code_id,
                tokens_per_power: Uint128::new(1000),
                min_bond: Uint128::new(1000),
                unbonding_periods: vec![
                    SECONDS_PER_DAY * 7,
                    SECONDS_PER_DAY * 14,
                    SECONDS_PER_DAY * 21,
                ],
                max_distributions: 6,
                converter: None,
            },
            trading_starts: None,
        },
        &[],
        "Wyndex Factory",
        None,
    )
    .unwrap()
}

// Utility function to initialize the wyndex multi-hop smart contract
fn init_multi_hop(
    app: &mut App,
    multi_hop_code_id: u64,
    owner: Addr,
    factory_address: Addr,
) -> Addr {
    app.instantiate_contract(
        multi_hop_code_id,
        owner,
        &MultiHopInstantiateMsg {
            wyndex_factory: factory_address.to_string(),
        },
        &[],
        "Wyndex Multi Hop",
        None,
    )
    .unwrap()
}

// -------------------------------------------------------------------------------------------------
// SuiteBuilder
// -------------------------------------------------------------------------------------------------

/// Stores a contract info.
#[cw_serde]
pub struct ContractInfo {
    pub address: Addr,
    pub code_id: u64,
}

/// Structure to be imported in a multitest environment to have access to Wyndex contracts.
pub struct WyndexSuiteBuilder {
    pub owner: Addr,
}

impl WyndexSuiteBuilder {
    pub fn init_wyndex(self, app: &mut App) -> WyndexSuite {
        // Store Wyndex contracts.
        let cw20_code_id = app.store_code(contract_cw20());
        let pair_lsd_code_id = app.store_code(contract_pair_lsd());
        let pair_code_id = app.store_code(contract_pair());
        let factory_code_id = app.store_code(contract_factory());
        let staking_code_id = app.store_code(contract_staking());
        let multi_hop_code_id = app.store_code(contract_multi_hop());
        let mock_hub_code_id = app.store_code(contract_mock_hub());

        // Initialize Wyndex contarcts.
        let mock_hub_addr = init_mock_hub(app, mock_hub_code_id, self.owner.clone());
        let factory_addr = init_factory(
            app,
            factory_code_id,
            self.owner.clone(),
            pair_lsd_code_id,
            pair_code_id,
            cw20_code_id,
            staking_code_id,
        );
        let multi_hop_addr = init_multi_hop(
            app,
            multi_hop_code_id,
            self.owner.clone(),
            factory_addr.clone(),
        );

        WyndexSuite {
            owner: self.owner,
            cw20_code_id,
            pair_lsd_code_id,
            pair_code_id,
            mock_hub: ContractInfo {
                address: mock_hub_addr,
                code_id: mock_hub_code_id,
            },
            factory: ContractInfo {
                address: factory_addr,
                code_id: factory_code_id,
            },
            multi_hop: ContractInfo {
                address: multi_hop_addr,
                code_id: multi_hop_code_id,
            },
        }
    }
}

/// Stores information related to a wyndex deployment.
pub struct WyndexSuite {
    pub owner: Addr,
    pub cw20_code_id: u64,
    pub pair_lsd_code_id: u64,
    pub pair_code_id: u64,
    pub mock_hub: ContractInfo,
    pub factory: ContractInfo,
    pub multi_hop: ContractInfo,
}

impl WyndexSuite {
    pub fn instantiate_token(&mut self, app: &mut App, token: &str) -> Addr {
        app.instantiate_contract(
            self.cw20_code_id,
            self.owner.clone(),
            &Cw20BaseInstantiateMsg {
                name: token.to_owned() + " token",
                symbol: token.to_owned(),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: self.owner.to_string(),
                    cap: None,
                }),
                marketing: None,
            },
            &[],
            token,
            None,
        )
        .unwrap()
    }

    /// Utility function to create a standard or an LSD pair. Returns the address of the newly
    /// created pair.
    pub fn create_pair(
        &mut self,
        app: &mut App,
        asset_infos: &[AssetInfo; 2],
        pair_type: PairType,
        init_params: Option<StablePoolParams>,
    ) -> Addr {
        let _: AppResponse = app
            .execute_contract(
                self.owner.clone(),
                self.factory.clone().address,
                &FactoryExecuteMsg::CreatePair {
                    pair_type,
                    asset_infos: asset_infos.to_vec(),
                    init_params: init_params.map(|p| to_binary(&p).unwrap()),
                    total_fee_bps: None,
                    staking_config: PartialStakeConfig::default(),
                },
                &[],
            )
            .unwrap();

        // Query the factory to obtain pair address.
        let res: PairInfo = app
            .wrap()
            .query_wasm_smart(
                self.factory.clone().address,
                &FactoryQueryMsg::Pair {
                    asset_infos: asset_infos.to_vec(),
                },
            )
            .unwrap();

        res.contract_addr
    }

    pub fn provide_liquidity(
        &mut self,
        app: &mut App,
        owner: &str,
        pair: &Addr,
        assets: &[Asset],
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        app.execute_contract(
            Addr::unchecked(owner),
            pair.clone(),
            &PairExecuteMsg::ProvideLiquidity {
                assets: assets.to_vec(),
                slippage_tolerance: None,
                receiver: None,
            },
            send_funds,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn swap<'a>(
        &mut self,
        app: &mut App,
        pair: &Addr,
        sender: &str,
        offer_asset: Asset,
        ask_asset_info: impl Into<Option<AssetInfo>>,
        belief_price: impl Into<Option<Decimal>>,
        max_spread: impl Into<Option<Decimal>>,
        to: impl Into<Option<&'a str>>,
    ) -> AnyResult<AppResponse> {
        match &offer_asset.info {
            AssetInfo::Token(address) => app.execute_contract(
                Addr::unchecked(sender),
                Addr::unchecked(address),
                &Cw20ExecuteMsg::Send {
                    contract: pair.to_string(),
                    amount: offer_asset.amount,
                    msg: to_binary(&Cw20HookMsg::Swap {
                        ask_asset_info: ask_asset_info.into(),
                        referral_address: None,
                        referral_commission: None,
                        belief_price: belief_price.into(),
                        max_spread: max_spread.into(),
                        to: to.into().map(|s| s.to_owned()),
                    })?,
                },
                &[],
            ),
            AssetInfo::Native(denom) => {
                let funds = &[coin(offer_asset.amount.u128(), denom)];
                app.execute_contract(
                    Addr::unchecked(sender),
                    pair.clone(),
                    &PairExecuteMsg::Swap {
                        offer_asset,
                        ask_asset_info: ask_asset_info.into(),
                        referral_address: None,
                        referral_commission: None,
                        belief_price: belief_price.into(),
                        max_spread: max_spread.into(),
                        to: to.into().map(|s| s.to_owned()),
                    },
                    funds,
                )
            }
        }
    }
}
