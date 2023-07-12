use anyhow::Result as AnyResult;
use cw20::{BalanceResponse, Cw20Coin, Cw20QueryMsg, MinterResponse};
use wyndex::oracle::SamplePeriod;
use std::collections::HashMap;
use wyndex::asset::AssetInfo;
use wyndex::pair::{LsdInfo, PairInfo, StablePoolParams};

use wyndex::factory::{PairType, QueryMsg as FactoryQueryMsg};
use wyndex_tests::builder::{WyndexSuite, WyndexSuiteBuilder};

use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, ContractInfoResponse, Decimal, StdResult, Uint128, Empty,
};
use cw_multi_test::{AppResponse, Contract, ContractWrapper, Executor, App};
use isotonic_market::msg::{
    ExecuteMsg as MarketExecuteMsg, MigrateMsg as MarketMigrateMsg, QueryMsg as MarketQueryMsg,
    ReceiveMsg as MarketReceiveMsg, TokensBalanceResponse,
};
use isotonic_market::state::SECONDS_IN_YEAR;

use wyndex_oracle::msg::{
    ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg,
    QueryMsg as OracleQueryMsg,
};

use utils::{credit_line::CreditLineResponse, interest::Interest, token::Token};

use cw20_base::msg::{ExecuteMsg as Cw20BaseExecuteMsg, InstantiateMsg as Cw20BaseInstantiateMsg};

use crate::msg::{
    ExecuteMsg, InstantiateMsg, IsOnMarketResponse, LiquidationResponse,
    ListEnteredMarketsResponse, ListMarketsResponse, MarketConfig, MarketResponse, QueryMsg,
    ReceiveMsg,
};
use crate::state::Config;

pub const DAY: u64 = 24 * 3600;
// Generic
pub const MARKET_TOKEN: &str = "market";
pub const COMMON: &str = "common";
// Native
pub const OSMO: &str = "osmo";
pub const JUNO: &str = "juno";
pub const ATOM: &str = "atom";
// Cw20 tokens
pub const WYND: &str = "wynd";
pub const DAODAO: &str = "daodao";
// Addresses
pub const OWNER: &str = "owner";
pub const LENDER: &str = "lender";
pub const LENDER_2: &str = "lender_2";
pub const BORROWER: &str = "borrower_1";
pub const BORROWER_2: &str = "borrower_2";
pub const ACTOR: &str = "actor";
pub const ACTOR_2: &str = "actor_2";
pub const GOVERNANCE: &str = "governance";
pub const LIQUIDATOR: &str = "liquidator";
pub const DEBTOR: &str = "debtor";
pub const DEPOSIT: &str = "deposit";
pub const DEPOSIT_2: &str = "deposit_2";

fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );

    Box::new(contract)
}

fn contract_oracle() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        wyndex_oracle::contract::execute,
        wyndex_oracle::contract::instantiate,
        wyndex_oracle::contract::query,
    );

    Box::new(contract)
}

/// Returns a wrapper around the isotonic-credit-agency contract.
fn contract_credit_agency() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);

    Box::new(contract)
}

/// Returns a wrapper around the isotonic-market contract.
pub fn contract_market() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        isotonic_market::contract::execute,
        isotonic_market::contract::instantiate,
        isotonic_market::contract::query,
    )
    .with_reply(isotonic_market::contract::reply)
    .with_migrate(isotonic_market::contract::migrate);

    Box::new(contract)
}

/// Returns a wrapper around the isotonic-token contract.
fn contract_token() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        isotonic_token::contract::execute,
        isotonic_token::contract::instantiate,
        isotonic_token::contract::query,
    );

    Box::new(contract)
}

// -------------------------------------------------------------------------------------------------
// Instantiate
// -------------------------------------------------------------------------------------------------

fn init_oracle(app: &mut App, oracle_id: u64, owner: Addr, multi_hop_address: Addr) -> Addr {
    let address = app
        .instantiate_contract(
            oracle_id,
            owner.clone(),
            &OracleInstantiateMsg {
                controller: owner.to_string(),
                multi_hop: multi_hop_address.to_string(),
                start_age: 1,
                sample_period: SamplePeriod::HalfHour,
            },
            &[],
            "Wyndex Oracle",
            Some(owner.to_string()),
        )
        .unwrap();

    address
}

// -------------------------------------------------------------------------------------------------
// SuiteBuilder
// -------------------------------------------------------------------------------------------------

/// Builder for test suite
#[derive(Debug)]
pub struct SuiteBuilder {
    /// Initial funds to provide for testing
    lsd_pools: HashMap<u64, (Coin, Coin)>,
    // Instantiate fields.
    gov_contract: String,
    reward_token: Token,
    /// Initial funds to provide for testing
    funds: Vec<(Addr, Vec<Coin>)>,
    liquidation_price: Decimal,
    common_token: Token,
    /// Native tokens pool created during Suite building. Cw20 tokens pools have to be created later
    /// with token addresses.
    pools: HashMap<u64, (utils::coin::Coin, utils::coin::Coin)>,
    borrow_limit_ratio: Decimal,
    initial_cw20: HashMap<String, Vec<Cw20Coin>>,
}

impl SuiteBuilder {
    // Default common and reward tokens are set to native.
    pub fn new() -> Self {
        Self {
            gov_contract: "owner".to_string(),
            reward_token: Token::Native("reward".to_owned()),
            funds: vec![],
            liquidation_price: Decimal::percent(92),
            common_token: Token::Native(COMMON.to_owned()),
            pools: HashMap::new(),
            lsd_pools: HashMap::new(),
            borrow_limit_ratio: Decimal::one(),
            initial_cw20: HashMap::new(),
        }
    }

    pub fn with_gov(mut self, gov: impl ToString) -> Self {
        self.gov_contract = gov.to_string();
        self
    }

    /// Helper to initialize the contract with a reward token.
    pub fn with_reward_token(mut self, token: Token) -> Self {
        self.reward_token = token;
        self
    }

    /// Sets initial amount of distributable tokens on address
    pub fn with_funds(mut self, addr: &str, funds: &[utils::coin::Coin]) -> Self {
        let native_funds = funds
            .iter()
            .map(|c| Coin::try_from(c.clone()).unwrap())
            .collect();
        self.funds.push((Addr::unchecked(addr), native_funds));
        self
    }

    pub fn with_initial_cw20(mut self, denom: String, (address, amount): (&str, u64)) -> Self {
        let initial_balance = Cw20Coin {
            address: address.to_owned(),
            amount: Uint128::from(amount),
        };

        self.initial_cw20
            .entry(denom)
            .and_modify(|l| l.push(initial_balance.clone()))
            .or_insert_with(|| vec![initial_balance]);
        self
    }

    pub fn with_liquidation_price(mut self, liquidation_price: Decimal) -> Self {
        self.liquidation_price = liquidation_price;
        self
    }

    /// Helper to initialize the contract with a native common token.
    pub fn with_common_token(mut self, common_token: Token) -> Self {
        self.common_token = common_token;
        self
    }

    pub fn with_pool(mut self, id: u64, pool: (utils::coin::Coin, utils::coin::Coin)) -> Self {
        if pool.0.denom.is_cw20() || pool.1.denom.is_cw20() {
            return self;
        }
        self.pools.insert(id, pool);
        self
    }

    /// Add info to create an lsd pool.
    pub fn with_lsd_pool(mut self, id: u64, pool: (Coin, Coin)) -> Self {
        self.lsd_pools.insert(id, pool);
        self
    }

    pub fn with_borrow_limit_ratio(mut self, limit: Decimal) -> Self {
        self.borrow_limit_ratio = limit;
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app = App::default();
        let owner = Addr::unchecked("owner");
        let common_token = self.common_token.clone();

        // Initialize wyndex test dependencies.
        let wyndex_builder = WyndexSuiteBuilder {
            owner: owner.clone(),
        };
        let mut wyndex_suite = wyndex_builder.init_wyndex(&mut app);

        /*
        let pair_addr = wyndex_suite.create_pair(&mut app, &[
            AssetInfo::Token(wyjuno_addr.to_string()),
            AssetInfo::Native("juno".to_string()),
            ],
        init_params,
        );
        */

        // Store Wyndlend contracts.
        let isotonic_market_id = app.store_code(contract_market());
        let isotonic_token_id = app.store_code(contract_token());
        let contract_id = app.store_code(contract_credit_agency());
        let oracle_id = app.store_code(contract_oracle());

        // Initialize Wyndlend contracts.
        let oracle_contract = init_oracle(
            &mut app,
            oracle_id,
            owner.clone(),
            wyndex_suite.multi_hop.clone().address,
        );
        let contract = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &InstantiateMsg {
                    gov_contract: self.gov_contract.clone(),
                    isotonic_market_id,
                    isotonic_token_id,
                    reward_token: self.reward_token,
                    common_token: self.common_token,
                    liquidation_price: self.liquidation_price,
                    borrow_limit_ratio: self.borrow_limit_ratio,
                },
                &[],
                "credit-agency",
                Some(owner.clone().to_string()),
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
            app,
            owner,
            gov_contract: Addr::unchecked(self.gov_contract),
            contract,
            common_token,
            oracle_contract,
            starting_pools: self.pools,
            wyndex_suite,
        }
    }
}

/// Test suite
pub struct Suite {
    /// The multitest app
    app: App,
    /// Contract's owner
    owner: Addr,
    /// Governance contract
    gov_contract: Addr,
    /// Address of the Credit Agency contract
    contract: Addr,
    /// Common token
    common_token: Token,
    /// Address of isotonic price oracle
    pub oracle_contract: Addr,
    /// The pool values as defined by the builder, useful for resetting
    starting_pools: HashMap<u64, (utils::coin::Coin, utils::coin::Coin)>,
    /// Wyndex test suite
    wyndex_suite: WyndexSuite,
}

impl Suite {
    pub fn app(&mut self) -> &mut App {
        &mut self.app
    }

    /*
    pub fn set_pool(
        &mut self,
        pools: &[(u64, (utils::coin::Coin, utils::coin::Coin))],
    ) -> AnyResult<()> {
        let owner = self.owner.clone();
        let oracle = self.oracle_contract.clone();

        self.app
            .init_modules(|router, _, storage| -> AnyResult<()> {
                for (pool_id, (coin1, coin2)) in pools {
                    router.custom.set_pool(
                        storage,
                        *pool_id,
                        &Pool::new(coin1.clone(), coin2.clone()),
                    )?;
                }

                Ok(())
            })
            .unwrap();
        for (pool_id, (coin1, coin2)) in pools {
            self.app
                .execute_contract(
                    owner.clone(),
                    oracle.clone(),
                    &OracleExecuteMsg::RegisterPool {
                        pool_id: *pool_id,
                        denom1: coin1.denom.clone(),
                        denom2: coin2.denom.clone(),
                    },
                    &[],
                )
                .unwrap();
        }
        Ok(())
    }
*/
/*
    /// Reset the pools to their initial values. Useful if we're using the
    /// pools, but want to maintain the same conversion ratios in tests for simplicity
    /// and predictable credit line values.
    pub fn reset_pools(&mut self) -> AnyResult<()> {
        let starting_pools = self.starting_pools.clone();
        self.app
            .init_modules(|router, _, storage| -> AnyResult<()> {
                for (pool_id, (coin1, coin2)) in starting_pools {
                    router
                        .custom
                        .set_pool(storage, pool_id, &Pool::new(coin1, coin2))?;
                }

                Ok(())
            })?;

        Ok(())
    }
 */
    pub fn advance_seconds(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.time = block.time.plus_seconds(seconds);
            block.height += std::cmp::max(1, seconds / 5); // block time
        });
    }

    /// Helper to create a new isotonic market by specifying its configuration.
    pub fn create_market(&mut self, caller: &str, cfg: MarketConfig) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(caller),
            self.contract.clone(),
            &ExecuteMsg::CreateMarket(cfg),
            &[],
        )
    }

    /// Helper function to create a market with most of its params fixed by this function.
    pub fn create_market_quick(
        &mut self,
        caller: &str,
        isotonic_token: &str,
        market_token: Token,
        collateral_ratio: impl Into<Option<Decimal>>,
        interest_rates: impl Into<Option<(Decimal, Decimal)>>,
        reserve_factor: impl Into<Option<Decimal>>,
    ) -> AnyResult<AppResponse> {
        self.create_market(
            caller,
            MarketConfig {
                name: isotonic_token.to_string(),
                symbol: isotonic_token.to_string(),
                decimals: 9,
                market_token,
                market_cap: None,
                interest_rate: match interest_rates.into() {
                    Some((base, slope)) => Interest::Linear { base, slope },
                    None => Interest::Linear {
                        base: Decimal::percent(3),
                        slope: Decimal::percent(20),
                    },
                },
                interest_charge_period: SECONDS_IN_YEAR as u64,
                collateral_ratio: collateral_ratio
                    .into()
                    .unwrap_or_else(|| Decimal::percent(50)),
                price_oracle: self.oracle_contract.to_string(),
                reserve_factor: reserve_factor.into().unwrap_or_else(|| Decimal::percent(0)),
            },
        )
    }

    pub fn enter_market(&mut self, market: &str, addr: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(market),
            self.contract.clone(),
            &ExecuteMsg::EnterMarket {
                account: addr.to_owned(),
            },
            &[],
        )
    }

    pub fn exit_market(&mut self, addr: &str, market: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(addr),
            self.contract.clone(),
            &ExecuteMsg::ExitMarket {
                market: market.to_owned(),
            },
            &[],
        )
    }

    pub fn common_token(&self) -> &Token {
        &self.common_token
    }

    /// Queries the Credit Agency contract for configuration
    pub fn query_config(&self) -> AnyResult<Config> {
        let resp: Config = self
            .app
            .wrap()
            .query_wasm_smart(self.contract.clone(), &QueryMsg::Configuration {})?;
        Ok(resp)
    }

    /// Queries the Credit Agency contract for market addr associated to `asset`.
    pub fn query_market(&self, asset: Token) -> AnyResult<MarketResponse> {
        let resp: MarketResponse = self.app.wrap().query_wasm_smart(
            self.contract.clone(),
            &QueryMsg::Market {
                market_token: asset,
            },
        )?;
        Ok(resp)
    }

    /// Queries all markets within agency and returns sum of credit lines
    pub fn query_total_credit_line(&self, account: &str) -> AnyResult<CreditLineResponse> {
        let resp: CreditLineResponse = self.app.wrap().query_wasm_smart(
            self.contract.clone(),
            &QueryMsg::TotalCreditLine {
                account: account.to_string(),
            },
        )?;
        Ok(resp)
    }

    pub fn assert_market(&self, asset: Token) {
        let res = self.query_market(asset.clone()).unwrap();
        assert_eq!(res.market_token, asset);

        // We query the supposed market contract address to make extra sure
        // it was instantiated properly and exists.
        let resp: isotonic_market::state::Config = self
            .app
            .wrap()
            .query_wasm_smart(
                res.market,
                &isotonic_market::msg::QueryMsg::Configuration {},
            )
            .unwrap();
        assert_eq!(resp.market_token, asset);
    }

    /// Queries the Credit Agency contract for a list of markets with pagination
    pub fn list_markets(&self) -> AnyResult<ListMarketsResponse> {
        self.list_markets_with_pagination(None, None)
    }

    /// Queries the Credit Agency contract for a list of markets with pagination
    pub fn list_markets_with_pagination(
        &self,
        start_after: impl Into<Option<Token>>,
        limit: impl Into<Option<u32>>,
    ) -> AnyResult<ListMarketsResponse> {
        let resp: ListMarketsResponse = self.app.wrap().query_wasm_smart(
            self.contract.clone(),
            &QueryMsg::ListMarkets {
                start_after: start_after.into(),
                limit: limit.into(),
            },
        )?;
        Ok(resp)
    }

    /// Deposit tokens on market selected by denom of Coin. It manages both native and cw20 tokens.
    pub fn deposit_tokens_on_market(
        &mut self,
        account: &str,
        tokens: utils::coin::Coin,
    ) -> AnyResult<AppResponse> {
        let market = self.query_market(tokens.denom.clone())?;

        if tokens.denom.is_native() {
            self.app.execute_contract(
                Addr::unchecked(account),
                market.market,
                &MarketExecuteMsg::Deposit {},
                &[Coin::try_from(tokens).unwrap()],
            )
        } else {
            let msg: Binary = to_binary(&MarketReceiveMsg::Deposit {})?;

            self.app.execute_contract(
                Addr::unchecked(account),
                Addr::unchecked(tokens.denom.denom()),
                &Cw20BaseExecuteMsg::Send {
                    contract: market.market.to_string(),
                    amount: tokens.amount,
                    msg,
                },
                &[],
            )
        }
    }

    /// Borrow tokens from market selected by denom and amount of Coin
    pub fn borrow_tokens_from_market(
        &mut self,
        account: &str,
        tokens: utils::coin::Coin,
    ) -> AnyResult<AppResponse> {
        let market = self.query_market(tokens.denom)?;

        self.app.execute_contract(
            Addr::unchecked(account),
            market.market,
            &MarketExecuteMsg::Borrow {
                amount: tokens.amount,
            },
            &[],
        )
    }

    /// Borrow tokens from market selected by denom and amount of Coin
    pub fn withdraw_tokens_from_market(
        &mut self,
        account: &str,
        tokens: utils::coin::Coin,
    ) -> AnyResult<AppResponse> {
        let market = self.query_market(tokens.denom)?;

        self.app.execute_contract(
            Addr::unchecked(account),
            market.market,
            &MarketExecuteMsg::Withdraw {
                amount: tokens.amount,
            },
            &[],
        )
    }

    pub fn liquidate(
        &mut self,
        sender: &str,
        account: &str,
        tokens: &[Coin],
        collateral_denom: Token,
    ) -> AnyResult<AppResponse> {
        let ca = self.contract.clone();

        self.app.execute_contract(
            Addr::unchecked(sender),
            ca,
            &ExecuteMsg::Liquidate {
                account: account.to_owned(),
                collateral_denom,
            },
            tokens,
        )
    }

    pub fn liquidate_with_cw20(
        &mut self,
        sender: &str,
        account: &str,
        tokens: utils::coin::Coin,
        collateral_denom: Token,
    ) -> AnyResult<AppResponse> {
        let msg: Binary = to_binary(&ReceiveMsg::Liquidate {
            account: account.to_owned(),
            collateral_denom,
        })?;

        self.app.execute_contract(
            Addr::unchecked(sender),
            Addr::unchecked(tokens.denom.denom()),
            &Cw20BaseExecuteMsg::Send {
                contract: self.contract.to_string(),
                amount: tokens.amount,
                msg,
            },
            &[],
        )
    }

    pub fn repay_tokens_on_market(
        &mut self,
        account: &str,
        tokens: utils::coin::Coin,
    ) -> AnyResult<AppResponse> {
        let market = self.query_market(tokens.denom.clone())?;

        use Token::*;
        match tokens.denom {
            Native(_) => self.app.execute_contract(
                Addr::unchecked(account),
                market.market,
                &MarketExecuteMsg::Repay {},
                &[Coin::try_from(tokens).unwrap()],
            ),
            Cw20(_) => {
                let msg: Binary = to_binary(&MarketReceiveMsg::Repay {})?;

                self.app.execute_contract(
                    Addr::unchecked(account),
                    Addr::unchecked(tokens.denom.denom()),
                    &Cw20BaseExecuteMsg::Send {
                        contract: market.market.to_string(),
                        amount: tokens.amount,
                        msg,
                    },
                    &[],
                )
            }
        }
    }

    pub fn repay_with_collateral(
        &mut self,
        sender: &str,
        max_collateral: utils::coin::Coin,
        amount_to_repay: utils::coin::Coin,
    ) -> AnyResult<AppResponse> {
        let ca = self.contract.clone();
        self.app.execute_contract(
            Addr::unchecked(sender),
            ca,
            &ExecuteMsg::RepayWithCollateral {
                max_collateral,
                amount_to_repay,
            },
            &[],
        )
    }

    pub fn list_entered_markets(
        &self,
        account: &str,
        start_after: impl Into<Option<String>>,
        limit: impl Into<Option<u32>>,
    ) -> AnyResult<Vec<Addr>> {
        let resp: ListEnteredMarketsResponse = self.app.wrap().query_wasm_smart(
            Addr::unchecked(&self.contract),
            &QueryMsg::ListEnteredMarkets {
                account: account.to_owned(),
                start_after: start_after.into(),
                limit: limit.into(),
            },
        )?;

        Ok(resp.markets)
    }

    pub fn list_all_entered_markets(&self, account: &str) -> AnyResult<Vec<Addr>> {
        self.list_entered_markets(account, None, None)
    }

    pub fn is_on_market(&self, account: &str, market: &str) -> AnyResult<bool> {
        let resp: IsOnMarketResponse = self.app.wrap().query_wasm_smart(
            Addr::unchecked(&self.contract),
            &QueryMsg::IsOnMarket {
                account: account.to_owned(),
                market: market.to_owned(),
            },
        )?;

        Ok(resp.participating)
    }

    /// Queries collateral and debt on market pointed by the token for given account
    pub fn query_tokens_balance(
        &self,
        market_token: Token,
        account: &str,
    ) -> AnyResult<TokensBalanceResponse> {
        let market = self.query_market(market_token)?;
        let resp: TokensBalanceResponse = self.app.wrap().query_wasm_smart(
            market.market,
            &MarketQueryMsg::TokensBalance {
                account: account.to_owned(),
            },
        )?;
        Ok(resp)
    }

    /// Queries configuration from market selected by token
    pub fn query_market_config(&self, token: Token) -> AnyResult<isotonic_market::state::Config> {
        let market = self.query_market(token)?;

        let resp: isotonic_market::state::Config = self
            .app
            .wrap()
            .query_wasm_smart(market.market, &MarketQueryMsg::Configuration {})?;
        Ok(resp)
    }

    pub fn query_contract_code_id(&mut self, asset: Token) -> AnyResult<u64> {
        use cosmwasm_std::{QueryRequest, WasmQuery};
        let market = self.query_market(asset)?;
        let query_result: ContractInfoResponse =
            self.app
                .wrap()
                .query(&QueryRequest::Wasm(WasmQuery::ContractInfo {
                    contract_addr: market.market.to_string(),
                }))?;
        Ok(query_result.code_id)
    }

    pub fn query_liquidation(&self, account: &str) -> AnyResult<LiquidationResponse> {
        let resp: LiquidationResponse = self.app.wrap().query_wasm_smart(
            Addr::unchecked(&self.contract),
            &QueryMsg::Liquidation {
                account: account.to_owned(),
            },
        )?;
        Ok(resp)
    }

    pub fn query_cw20_balance(&self, owner: &str, contract: String) -> StdResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            contract,
            &Cw20QueryMsg::Balance {
                address: owner.to_owned(),
            },
        )?;

        Ok(balance.balance.into())
    }

    pub fn sudo_adjust_market_id(&mut self, new_market_id: u64) -> AnyResult<AppResponse> {
        let contract = self.contract.clone();
        self.app.execute_contract(
            self.gov_contract.clone(),
            contract,
            &ExecuteMsg::AdjustMarketId { new_market_id },
            &[],
        )
    }

    pub fn sudo_adjust_token_id(&mut self, new_token_id: u64) -> AnyResult<AppResponse> {
        let contract = self.contract.clone();
        self.app.execute_contract(
            self.gov_contract.clone(),
            contract,
            &ExecuteMsg::AdjustTokenId { new_token_id },
            &[],
        )
    }

    pub fn sudo_adjust_common_token(&mut self, new_common_token: Token) -> AnyResult<AppResponse> {
        let contract = self.contract.clone();
        self.app.execute_contract(
            self.gov_contract.clone(),
            contract,
            &ExecuteMsg::AdjustCommonToken { new_common_token },
            &[],
        )
    }

    pub fn sudo_migrate_market(
        &mut self,
        market: &str,
        migrate_msg: MarketMigrateMsg,
    ) -> AnyResult<AppResponse> {
        let contract = self.contract.clone();
        self.app.execute_contract(
            self.gov_contract.clone(),
            contract,
            &ExecuteMsg::MigrateMarket {
                contract: market.to_owned(),
                migrate_msg,
            },
            &[],
        )
    }
}
