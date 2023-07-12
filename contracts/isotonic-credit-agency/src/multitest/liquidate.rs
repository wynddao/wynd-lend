use super::suite::{SuiteBuilder, COMMON};
use crate::multitest::suite::{ACTOR, ATOM, DEBTOR, GOVERNANCE, JUNO, LIQUIDATOR, OSMO, WYND};
use crate::{error::ContractError, msg::MarketConfig};

use isotonic_token::error::ContractError as TokenContractError;

use cosmwasm_std::{Decimal, Uint128};

use utils::credit_line::{CreditLineResponse, CreditLineValues};
use utils::token::Token;

const YEAR_IN_SECONDS: u64 = 365 * 24 * 3600;

#[test]
fn send_more_then_one_denom() {
    // This test is only for native since no multiple cw20 can be sent at once.
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(OSMO.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(
            LIQUIDATOR,
            &[
                native_token_1.clone().into_coin(500u128),
                native_token_2.clone().into_coin(500u128),
            ],
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_1.denom(),
            native_token_1.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    let err = suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[
                native_token_1
                    .clone()
                    .into_coin(100u128)
                    .try_into()
                    .unwrap(),
                native_token_2.into_coin(100u128).try_into().unwrap(),
            ],
            native_token_1,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::LiquidationOnlyOneDenomRequired {},
        err.downcast().unwrap(),
    );
}

#[test]
fn account_doesnt_have_debt_bigger_then_credit_line() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LIQUIDATOR, &[native_token.clone().into_coin(5_000u128)])
        .with_funds(DEBTOR, &[native_token.clone().into_coin(500u128)])
        .with_liquidation_price(Decimal::percent(92))
        .with_pool(
            1,
            (
                common_token.into_coin(100u128),
                native_token.clone().into_coin(100u128),
            ),
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            Decimal::percent(80),
            None,
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEBTOR, native_token.clone().into_coin(500u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(500),
            credit_line: Uint128::new(400),
            borrow_limit: Uint128::new(400),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );

    // debt must be higher then credit line, so 400 debt with 400 credit line won't allow liquidation
    suite
        .borrow_tokens_from_market(DEBTOR, native_token.clone().into_coin(400u128))
        .unwrap();
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert!(matches!(
        total_credit_line,
        CreditLineResponse {
            debt,
            ..
        } if debt.amount == Uint128::new(400)));

    let err = suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[native_token.clone().into_coin(400u128).try_into().unwrap()],
            native_token,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::LiquidationNotAllowed {},
        err.downcast().unwrap()
    );
}

#[test]
fn liquidating_whole_debt_native() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LIQUIDATOR, &[native_token.clone().into_coin(5_000u128)])
        .with_funds(DEBTOR, &[native_token.clone().into_coin(600u128)])
        .with_liquidation_price(Decimal::percent(92))
        .with_pool(
            1,
            (
                common_token.into_coin(100u128),
                native_token.clone().into_coin(100u128),
            ),
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            Decimal::percent(80),
            None,
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEBTOR, native_token.clone().into_coin(500u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(500),
            credit_line: Uint128::new(400),
            borrow_limit: Uint128::new(400),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );

    // debt must be higher then credit line, but DEBTOR can borrow at most 400 tokens
    suite
        .borrow_tokens_from_market(DEBTOR, native_token.clone().into_coin(400u128))
        .unwrap();
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert!(matches!(
        total_credit_line,
        CreditLineResponse {
            debt,
            ..
        } if debt.amount == Uint128::new(400)));

    suite.advance_seconds(YEAR_IN_SECONDS);

    // Repay some tokens to trigger interest rate charges
    suite
        .repay_tokens_on_market(DEBTOR, native_token.clone().into_coin(2u128))
        .unwrap();

    // utilisation is 80% (400/500)
    // default interest rates are 3% with 20% slope which gives 3% + 20% * 80% = 19%
    // after a year debt increases to 473.63 tokens
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(576),
            credit_line: Uint128::new(460),
            borrow_limit: Uint128::new(460),
            debt: Uint128::new(474)
        }
        .make_response(suite.common_token().clone())
    );

    suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[native_token.clone().into_coin(474u128).try_into().unwrap()],
            native_token,
        )
        .unwrap();

    // Liquidation price is 0.92
    // Repaid value is 474 * 1.0 (oracle's price for same denom) * 0.92 = 515.22
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 575 - 515 = 60
            collateral: Uint128::new(61),
            credit_line: Uint128::new(48),
            borrow_limit: Uint128::new(48),
            debt: Uint128::new(0)
        }
        .make_response(suite.common_token().clone())
    );

    let total_credit_line = suite.query_total_credit_line(LIQUIDATOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 515 tokens transferred as reward from DEBTOR
            collateral: Uint128::new(514), // FIXME: Rounding issue? Message debug shows 515 transferred
            credit_line: Uint128::new(411),
            borrow_limit: Uint128::new(411),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn liquidating_whole_debt_cw20() {
    let common_token = Token::Native(COMMON.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_initial_cw20(cw20_token.denom(), (LIQUIDATOR, 5_000))
        .with_initial_cw20(cw20_token.denom(), (DEBTOR, 600))
        .with_liquidation_price(Decimal::percent(92))
        .build();

    let cw20_token = suite
        .starting_cw20
        .get(&cw20_token.denom())
        .unwrap()
        .clone();

    // create cw20 tokens pools
    suite
        .set_pool(&[(
            1,
            (
                common_token.into_coin(100u128),
                cw20_token.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token.denom(),
            cw20_token.clone(),
            Decimal::percent(80),
            None,
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEBTOR, cw20_token.clone().into_coin(500u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(500),
            credit_line: Uint128::new(400),
            borrow_limit: Uint128::new(400),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );

    // debt must be higher then credit line, but DEBTOR can borrow at most 400 tokens
    suite
        .borrow_tokens_from_market(DEBTOR, cw20_token.clone().into_coin(400u128))
        .unwrap();
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert!(matches!(
        total_credit_line,
        CreditLineResponse {
            debt,
            ..
        } if debt.amount == Uint128::new(400)));

    suite.advance_seconds(YEAR_IN_SECONDS);

    // Repay some tokens to trigger interest rate charges
    suite
        .repay_tokens_on_market(DEBTOR, cw20_token.clone().into_coin(2u128))
        .unwrap();

    // utilisation is 80% (400/500)
    // default interest rates are 3% with 20% slope which gives 3% + 20% * 80% = 19%
    // after a year debt increases to 473.63 tokens
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(576),
            credit_line: Uint128::new(460),
            borrow_limit: Uint128::new(460),
            debt: Uint128::new(474)
        }
        .make_response(suite.common_token().clone())
    );

    suite
        .liquidate_with_cw20(
            LIQUIDATOR,
            DEBTOR,
            cw20_token.clone().into_coin(474u128),
            cw20_token,
        )
        .unwrap();

    // Liquidation price is 0.92
    // Repaid value is 474 * 1.0 (oracle's price for same denom) * 0.92 = 515.22
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 575 - 515 = 60
            collateral: Uint128::new(61),
            credit_line: Uint128::new(48),
            borrow_limit: Uint128::new(48),
            debt: Uint128::new(0)
        }
        .make_response(suite.common_token().clone())
    );

    let total_credit_line = suite.query_total_credit_line(LIQUIDATOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 515 tokens transferred as reward from DEBTOR
            collateral: Uint128::new(514), // FIXME: Rounding issue? Message debug shows 515 transferred
            credit_line: Uint128::new(411),
            borrow_limit: Uint128::new(411),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn receive_reward_in_different_denom_fails_if_theres_no_reward_market() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());
    let reward_token = Token::Native(ATOM.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LIQUIDATOR, &[native_token.clone().into_coin(5_000u128)])
        .with_funds(DEBTOR, &[native_token.clone().into_coin(600u128)])
        .with_liquidation_price(Decimal::percent(92))
        .with_pool(
            1,
            (
                common_token.clone().into_coin(100u128),
                native_token.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_token.into_coin(100u128),
                reward_token.clone().into_coin(150u128),
            ),
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            Decimal::percent(80),
            None,
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEBTOR, native_token.clone().into_coin(500u128))
        .unwrap();

    suite
        .borrow_tokens_from_market(DEBTOR, native_token.clone().into_coin(400u128))
        .unwrap();

    suite.advance_seconds(YEAR_IN_SECONDS);

    // Repay some tokens to trigger interest rate charges
    suite
        .repay_tokens_on_market(DEBTOR, native_token.clone().into_coin(2u128))
        .unwrap();

    let err = suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[native_token.into_coin(474u128).try_into().unwrap()],
            reward_token.clone(),
        )
        .unwrap_err();

    assert_eq!(
        ContractError::NoMarket(reward_token.denom()),
        err.downcast().unwrap()
    );
}

#[test]
fn receive_reward_different_denom_fails_if_debtor_has_not_enough_reward_tokens() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());
    let reward_token = Token::Native(ATOM.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LIQUIDATOR, &[native_token.clone().into_coin(5_000u128)])
        .with_funds(
            DEBTOR,
            &[
                native_token.clone().into_coin(600u128),
                reward_token.clone().into_coin(500u128),
            ],
        )
        .with_liquidation_price(Decimal::percent(92))
        .with_pool(
            1,
            (
                common_token.clone().into_coin(100u128),
                native_token.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_token.clone().into_coin(25u128),
                reward_token.clone().into_coin(100u128),
            ),
        )
        .build();

    // create market with very high interest rates
    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            Decimal::percent(80),
            (Decimal::percent(80), Decimal::percent(45)),
            None,
        )
        .unwrap();

    // create reward_denom market
    suite
        .create_market_quick(
            GOVERNANCE,
            &reward_token.denom(),
            reward_token.clone(),
            Decimal::percent(80),
            None,
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEBTOR, native_token.clone().into_coin(500u128))
        .unwrap();

    suite
        .borrow_tokens_from_market(DEBTOR, native_token.clone().into_coin(400u128))
        .unwrap();

    suite.advance_seconds(YEAR_IN_SECONDS);

    suite
        .set_pool(&[(
            2,
            (
                common_token.into_coin(150u128),
                reward_token.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    // Repay some tokens to trigger interest rate charges
    suite
        .repay_tokens_on_market(DEBTOR, native_token.clone().into_coin(10u128))
        .unwrap();

    // DEBTOR deposits some tokens in reward_denom market
    suite
        .deposit_tokens_on_market(DEBTOR, reward_token.clone().into_coin(50u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(1039),
            credit_line: Uint128::new(831),
            borrow_limit: Uint128::new(831),
            debt: Uint128::new(854)
        }
        .make_response(suite.common_token().clone())
    );

    let err = suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[native_token.into_coin(100u128).try_into().unwrap()],
            reward_token,
        )
        .unwrap_err();

    // Transferable amount is available balance / collateral ratio
    // balance = credit line - debt / price ratio = 830 - 755 (855 - 100 liquidated) / 1.5 = 50
    assert_eq!(
        TokenContractError::InsufficientTokens {
            available: Uint128::new(5_000_000),
            needed: Uint128::new(7_200_000)
        },
        err.downcast().unwrap()
    );
}

#[test]
fn receive_reward_in_different_denoms_no_interest_rates() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(ATOM.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LIQUIDATOR, &[native_token_1.clone().into_coin(160_000u128)])
        .with_funds(DEBTOR, &[native_token_2.clone().into_coin(5_000u128)])
        .with_liquidation_price(Decimal::percent(92))
        .with_pool(
            1,
            (
                common_token.clone().into_coin(400u128),
                native_token_2.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_token.clone().into_coin(10u128),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        .build();

    // create market atom osmo
    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_2.denom(),
            native_token_2.clone(),
            Decimal::percent(50),                        // collateral price
            (Decimal::percent(3), Decimal::percent(20)), // interest rates (base, slope)
            None,
        )
        .unwrap();

    // create ust market eth
    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_1.denom(),
            native_token_1.clone(),
            Decimal::percent(60),                        // collateral price
            (Decimal::percent(3), Decimal::percent(20)), // interest rates (base, slope)
            None,
        )
        .unwrap();

    // DEBTOR deposits 4000 atom
    suite
        .deposit_tokens_on_market(DEBTOR, native_token_2.clone().into_coin(4_000u128))
        .unwrap();

    // LIQUIDATOR deposits 100000 ust
    suite
        .deposit_tokens_on_market(LIQUIDATOR, native_token_1.clone().into_coin(100_000u128))
        .unwrap();

    // DEBTOR borrows 75_000 ust
    suite
        .borrow_tokens_from_market(DEBTOR, native_token_1.clone().into_coin(75_000u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(16000), // 4000 deposited * 4.0
            credit_line: Uint128::new(8000),
            borrow_limit: Uint128::new(8000), // 16000 collateral * 0.5 collateral price
            debt: Uint128::new(7500)          // 75_000 * 0.1
        }
        .make_response(suite.common_token().clone())
    );

    suite
        .set_pool(&[(
            1,
            (
                common_token.into_coin(300u128),
                native_token_2.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(12000), // 4000 deposited * 3.0
            credit_line: Uint128::new(6000),
            borrow_limit: Uint128::new(6000), // 12000 collateral * 0.5 collateral price
            debt: Uint128::new(7500)          // 75_000 * 0.1
        }
        .make_response(suite.common_token().clone())
    );

    // query liquidation data
    let liquidation_data = suite.query_liquidation(DEBTOR).unwrap();
    assert!(liquidation_data.can_liquidate);
    assert_eq!(
        liquidation_data.collateral[0].1,
        native_token_2.clone().into_coin(4_000u128)
    );
    assert_eq!(
        liquidation_data.debt[0].1,
        native_token_1.clone().into_coin(75_000u128)
    );

    // successful liquidation of 6000 tokens
    suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[native_token_1
                .clone()
                .into_coin(60_000u128)
                .try_into()
                .unwrap()],
            native_token_2.clone(),
        )
        .unwrap();

    // Liquidation price is 0.92
    // Repaid value is 60_000 ust * 0.1 / 3.0 / 0.92 = 2000 / 0.92 ~= 1999 / 0.92 = 2173 LATOM
    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // (4000 deposited - 2173 repaid) * 3.0 tokens price = 1827 * 3.0
            collateral: Uint128::new(5481),
            // 5481 * 0.5 collateral price
            credit_line: Uint128::new(2740),
            borrow_limit: Uint128::new(2740),
            // 7500 - (60_000 * 0.1)
            debt: Uint128::new(1500),
        }
        .make_response(suite.common_token().clone())
    );
    let balance = suite.query_tokens_balance(native_token_1, DEBTOR).unwrap();
    assert_eq!(balance.debt.amount, Uint128::new(15000)); // 1500 / 0.1 price
    let balance = suite
        .query_tokens_balance(native_token_2.clone(), DEBTOR)
        .unwrap();
    assert_eq!(balance.collateral.amount, Uint128::new(1827)); // (4000 deposited - 2173 repaid)

    let total_credit_line = suite.query_total_credit_line(LIQUIDATOR).unwrap();
    assert!(matches!(
        total_credit_line,
        CreditLineResponse {
            collateral,
            ..
        // deposited 100_000 * 0.1 + repaid 2173 * 3.0 (actually 2172 - FIXME rounding error)
        } if collateral.amount == Uint128::new(16_519)
    ));
    let balance = suite
        .query_tokens_balance(native_token_2, LIQUIDATOR)
        .unwrap();
    assert_eq!(balance.collateral.amount, Uint128::new(2173)); // 2173 repaid
}

#[test]
fn receive_reward_in_different_denoms_with_six_months_interests() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(ATOM.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LIQUIDATOR, &[native_token_1.clone().into_coin(100_000u128)])
        .with_funds(DEBTOR, &[native_token_2.clone().into_coin(5_000u128)])
        .with_funds(
            ACTOR,
            &[
                native_token_1.clone().into_coin(100_000u128),
                native_token_2.clone().into_coin(10_000u128),
            ],
        )
        .with_liquidation_price(Decimal::percent(92))
        .with_pool(
            1,
            (
                common_token.clone().into_coin(400u128),
                native_token_2.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_token.clone().into_coin(10u128),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        .build();

    suite
        .create_market(
            GOVERNANCE,
            MarketConfig {
                name: ("c".to_owned() + &native_token_2.denom()),
                symbol: ("c".to_owned() + &native_token_2.denom()),
                decimals: 9,
                market_token: native_token_2.clone(),
                market_cap: None,
                interest_rate: utils::interest::Interest::Linear {
                    base: Decimal::percent(3),
                    slope: Decimal::percent(20),
                },
                interest_charge_period: YEAR_IN_SECONDS / 2,
                collateral_ratio: Decimal::percent(50),
                price_oracle: suite.oracle_contract.to_string(),
                reserve_factor: Decimal::percent(0),
            },
        )
        .unwrap();

    suite
        .create_market(
            GOVERNANCE,
            MarketConfig {
                name: ("c".to_owned() + &native_token_1.denom()),
                symbol: ("c".to_owned() + &native_token_1.denom()),
                decimals: 9,
                market_token: native_token_1.clone(),
                market_cap: None,
                interest_rate: utils::interest::Interest::Linear {
                    base: Decimal::percent(3),
                    slope: Decimal::percent(20),
                },
                interest_charge_period: YEAR_IN_SECONDS / 2,
                collateral_ratio: Decimal::percent(60),
                price_oracle: suite.oracle_contract.to_string(),
                reserve_factor: Decimal::percent(0),
            },
        )
        .unwrap();

    // investments from the others
    suite
        .deposit_tokens_on_market(ACTOR, native_token_2.clone().into_coin(10_000u128))
        .unwrap();
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(100_000u128))
        .unwrap();
    suite
        .borrow_tokens_from_market(ACTOR, native_token_2.clone().into_coin(2_000u128))
        .unwrap();
    suite
        .borrow_tokens_from_market(ACTOR, native_token_1.clone().into_coin(20_000u128))
        .unwrap();

    // DEBTOR deposits 4000 atom
    suite
        .deposit_tokens_on_market(DEBTOR, native_token_2.clone().into_coin(4_000u128))
        .unwrap();
    // DEBTOR borrows 75_000 ust
    suite
        .borrow_tokens_from_market(DEBTOR, native_token_1.clone().into_coin(75_000u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(16000), // 4000 deposited * 4.0
            credit_line: Uint128::new(8000),
            borrow_limit: Uint128::new(8000), // 16000 collateral * 0.5 collateral price
            debt: Uint128::new(7500)          // 75_000 * 0.1
        }
        .make_response(suite.common_token().clone())
    );

    suite.advance_seconds(YEAR_IN_SECONDS / 2);

    // change ATOM price to 3.0 per common denom
    suite
        .set_pool(&[(
            1,
            (
                common_token.into_coin(300u128),
                native_token_2.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    // current interest rates
    // rates = (base + slope * utilization) / 2 (half year)
    // atom = (3% + 20% * (2000/(10_000 + 4000))) / 2 = (3% + 20% * 14.3%) / 2 = (3% + 2.8%) / 2 = 2.9% ~= 3%
    // ust = (3% + 20% * ((20_000 + 75_000)/100_000)) / 2 = (3% + 20% * 95%) / 2 = (3% + 19%) / 2 = 11%

    // expected numbers before liquidation
    // LATOM = 4000 + (2000 * 0.03 * 4000/14000) = 4017
    // BUST = 75_000 * 1.11 * 0.1 = 8325

    // successful liquidation of 6000 tokens
    suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[native_token_1
                .clone()
                .into_coin(60_000u128)
                .try_into()
                .unwrap()],
            native_token_2.clone(),
        )
        .unwrap();

    // Liquidation price is 0.92
    // Repaid value is 60_000 ust * 0.1 / 3.0 / 0.92 = 2000 / 0.92 ~= 1999 / 0.92 = 2172 LATO

    let balance = suite.query_tokens_balance(native_token_1, DEBTOR).unwrap();
    // 75_000 * 1.11 (interests) - 60_000 (repaid) = 83250 - 60000
    assert_eq!(balance.debt.amount, Uint128::new(23250));
    let balance = suite
        .query_tokens_balance(native_token_2.clone(), DEBTOR)
        .unwrap();
    // amount left after paying liquidation reward
    // 4017 - 2172 repaid = 1845 FIXME: rounding issue
    assert_eq!(balance.collateral.amount, Uint128::new(1843));

    let balance = suite
        .query_tokens_balance(native_token_2, LIQUIDATOR)
        .unwrap();
    assert_eq!(balance.collateral.amount, Uint128::new(2172)); // repaid amount as reward

    let total_credit_line = suite.query_total_credit_line(DEBTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 1843 * 3 = 5529
            collateral: Uint128::new(5529),
            // 5529 * 0.5 collateral price
            credit_line: Uint128::new(2764),
            borrow_limit: Uint128::new(2764),
            // 8375 - (60_000 * 0.1)
            debt: Uint128::new(2325),
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn liquidate_with_query_amounts() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(ATOM.to_owned());
    let native_token_3 = Token::Native(OSMO.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LIQUIDATOR, &[native_token_1.clone().into_coin(100_000u128)])
        .with_funds(DEBTOR, &[native_token_2.clone().into_coin(5_000u128)])
        .with_funds(
            ACTOR,
            &[
                native_token_1.clone().into_coin(100_000u128),
                native_token_2.clone().into_coin(10_000u128),
                native_token_3.clone().into_coin(100_000u128),
            ],
        )
        .with_liquidation_price(Decimal::percent(92))
        .with_pool(
            1,
            (
                common_token.clone().into_coin(1_000u128),
                native_token_2.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_token.clone().into_coin(10u128),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            3,
            (
                common_token.clone().into_coin(100u128),
                native_token_3.clone().into_coin(100u128),
            ),
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_1.denom(),
            native_token_1.clone(),
            None,
            None,
            None,
        )
        .unwrap();
    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_2.denom(),
            native_token_2.clone(),
            None,
            None,
            None,
        )
        .unwrap();
    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_3.denom(),
            native_token_3.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // investments from the others
    suite
        .deposit_tokens_on_market(ACTOR, native_token_2.clone().into_coin(10_000u128))
        .unwrap();
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(100_000u128))
        .unwrap();
    suite
        .deposit_tokens_on_market(ACTOR, native_token_3.clone().into_coin(100_000u128))
        .unwrap();

    // DEBTOR deposits 1000 atom, so credit line is 500 * 10.0 = 5000
    suite
        .deposit_tokens_on_market(DEBTOR, native_token_2.clone().into_coin(1_000u128))
        .unwrap();
    // DEBTOR borrows 25_000 ust and 2500 eth (exactly up to credit line)
    suite
        .borrow_tokens_from_market(DEBTOR, native_token_1.clone().into_coin(25_000u128))
        .unwrap();
    suite
        .borrow_tokens_from_market(DEBTOR, native_token_3.clone().into_coin(2_500u128))
        .unwrap();

    // make sure liquidation query returns same numbers we deposited / borrowed
    let liquidation_data = suite.query_liquidation(DEBTOR).unwrap();
    assert!(!liquidation_data.can_liquidate);
    assert_eq!(
        liquidation_data.debt[0].1,
        native_token_1.into_coin(25_000u128)
    );
    assert_eq!(
        liquidation_data.debt[1].1,
        native_token_3.into_coin(2_500u128)
    );
    assert_eq!(
        liquidation_data.collateral[0].1,
        native_token_2.clone().into_coin(1_000u128)
    );

    // change ATOM price to 8.0 per common denom
    suite
        .set_pool(&[(
            1,
            (
                common_token.into_coin(800u128),
                native_token_2.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    // should be allowed to liquidate now
    let liquidation_data = suite.query_liquidation(DEBTOR).unwrap();
    assert!(liquidation_data.can_liquidate);
    // no need to check amounts, they are the same as before

    // successful liquidation of 25_000 ust
    suite
        .liquidate(
            LIQUIDATOR,
            DEBTOR,
            &[liquidation_data.debt[0].1.clone().try_into().unwrap()],
            native_token_2,
        )
        .unwrap();

    let liquidation_data = suite.query_liquidation(DEBTOR).unwrap();
    assert!(!liquidation_data.can_liquidate);
    assert_eq!(
        liquidation_data.debt.len(),
        1,
        "should have paid off ust debt completely"
    );
}
