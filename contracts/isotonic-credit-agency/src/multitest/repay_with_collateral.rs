use super::suite::{SuiteBuilder, COMMON};
use crate::{
    error::ContractError,
    multitest::suite::{
        ACTOR, ATOM, DAODAO, DEPOSIT, DEPOSIT_2, GOVERNANCE, JUNO, LIQUIDATOR, OSMO, WYND,
    },
};

use cosmwasm_std::{Addr, Decimal, Uint128};
use utils::{assert_approx_eq, credit_line::CreditLineValues, token::Token};

#[test]
fn not_on_market() {
    // No need to test also with cw20 since no need to interact with its contract.
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(OSMO.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_1.denom(),
            native_token_1.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    let market_1 = suite.query_market(native_token_1.clone()).unwrap().market;

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_2.denom(),
            native_token_2.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    let market_2 = suite.query_market(native_token_2.clone()).unwrap().market;

    let err = suite
        .repay_with_collateral(
            ACTOR,
            native_token_1.clone().into_coin(1_000_000u128),
            native_token_2.clone().into_coin(1_000_000u128),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::NotOnMarket {
            address: Addr::unchecked(ACTOR),
            market: market_1.clone()
        },
        err.downcast().unwrap(),
        "expect to fail since the actor is not in the collateral market"
    );
    suite.enter_market(market_1.as_str(), ACTOR).unwrap();

    let err = suite
        .repay_with_collateral(
            ACTOR,
            native_token_1.into_coin(1_000_000u128),
            native_token_2.into_coin(1_000_000u128),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::NotOnMarket {
            address: Addr::unchecked(ACTOR),
            market: market_2
        },
        err.downcast().unwrap(),
        "expect to fail since the actor is not in the repay market"
    );
}

#[test]
fn simulated_debt_bigger_then_credit_line() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(ATOM.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(DEPOSIT, &[native_token_1.clone().into_coin(10_000_000u128)])
        .with_funds(
            DEPOSIT_2,
            &[native_token_2.clone().into_coin(10_000_000u128)],
        )
        .with_funds(ACTOR, &[native_token_1.clone().into_coin(5_000_000u128)])
        .with_pool(
            1,
            (
                common_token.clone().into_coin(10_000_000u128),
                native_token_1.clone().into_coin(10_000_000u128),
            ), // 1.0
        )
        .with_pool(
            2,
            (
                common_token.into_coin(5_000_000u128),
                native_token_2.clone().into_coin(10_000_000u128),
            ), // 0.5
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_1.denom(),
            native_token_1.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_2.denom(),
            native_token_2.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEPOSIT, native_token_1.clone().into_coin(10_000_000u128))
        .unwrap();
    suite
        .deposit_tokens_on_market(DEPOSIT_2, native_token_2.clone().into_coin(10_000_000u128))
        .unwrap();

    // 1_000_000 in terms of common token
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(1_000_000u128))
        .unwrap();
    // 500_000 in terms of common token
    suite
        .borrow_tokens_from_market(ACTOR, native_token_2.clone().into_coin(1_000_000u128))
        .unwrap();

    // Try to repay not-whole-loan using all your collateral
    let err = suite
        .repay_with_collateral(
            ACTOR,
            native_token_1.into_coin(1_000_000u128), // 1_000_000 common
            native_token_2.into_coin(800_000u128),   // 400_000 common
        )
        .unwrap_err();
    assert_eq!(
        ContractError::RepayingLoanUsingCollateralFailed {},
        err.downcast().unwrap(),
        "expected to fail because debt cannot be higher than collateral"
    );
}

#[test]
fn on_two_markets_native() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(ATOM.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(DEPOSIT, &[native_token_1.clone().into_coin(10_000_000u128)])
        .with_funds(
            DEPOSIT_2,
            &[native_token_2.clone().into_coin(10_000_000u128)],
        )
        .with_funds(ACTOR, &[native_token_1.clone().into_coin(5_000_000u128)])
        .with_pool(
            1,
            (
                common_token.clone().into_coin(20_000_000u128),
                native_token_1.clone().into_coin(10_000_000u128),
            ), // 2.0
        )
        .with_pool(
            2,
            (
                common_token.into_coin(5_000_000u128),
                native_token_2.clone().into_coin(10_000_000u128),
            ),
        ) // 0.5
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_1.denom(),
            native_token_1.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token_2.denom(),
            native_token_2.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEPOSIT, native_token_1.clone().into_coin(10_000_000u128))
        .unwrap();
    suite
        .deposit_tokens_on_market(DEPOSIT_2, native_token_2.clone().into_coin(10_000_000u128))
        .unwrap();

    // User creates a credit line through collateral
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(1_000_000u128))
        .unwrap();
    // User goes into debt, but is still liquid
    suite
        .borrow_tokens_from_market(ACTOR, native_token_2.clone().into_coin(1_000_000u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(ACTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 1_000_000 OSMO deposited * 2.0 oracle's price
            collateral: Uint128::new(2_000_000),
            // 1000_000 OSMO collateral * 2.0 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(1_000_000),
            borrow_limit: Uint128::new(1_000_000),
            // 1_000_000 ETH borrowed * 0.5 oracle's price
            debt: Uint128::new(500_000)
        }
        .make_response(suite.common_token().clone())
    );

    suite
        .repay_with_collateral(
            ACTOR,
            native_token_1.into_coin(1_000_000u128),
            native_token_2.into_coin(1_000_000u128),
        )
        .unwrap();
    let total_credit_line = suite.query_total_credit_line(ACTOR).unwrap();
    suite.reset_pools().unwrap();

    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(1_346_662),
            credit_line: Uint128::new(673_331),
            borrow_limit: Uint128::new(673_331),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn on_two_markets_cw20() {
    let common_token = Token::Native(COMMON.to_owned());
    let cw20_token_1 = Token::Cw20(WYND.to_owned());
    let cw20_token_2 = Token::Cw20(DAODAO.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_initial_cw20(cw20_token_1.denom(), (DEPOSIT, 10_000_000))
        .with_initial_cw20(cw20_token_1.denom(), (ACTOR, 5_000_000))
        .with_initial_cw20(cw20_token_2.denom(), (DEPOSIT_2, 10_000_000))
        .build();

    // recover the cw20 addresses
    let cw20_token_1 = suite
        .starting_cw20
        .get(&cw20_token_1.denom())
        .unwrap()
        .clone();
    let cw20_token_2 = suite
        .starting_cw20
        .get(&cw20_token_2.denom())
        .unwrap()
        .clone();

    // create cw20 tokens pools
    suite
        .set_pool(&[(
            1,
            (
                common_token.clone().into_coin(20_000_000u128),
                cw20_token_1.clone().into_coin(10_000_000u128),
            ),
        )])
        .unwrap();
    suite
        .set_pool(&[(
            2,
            (
                common_token.into_coin(5_000_000u128),
                cw20_token_2.clone().into_coin(10_000_000u128),
            ),
        )])
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token_1.denom(),
            cw20_token_1.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token_2.denom(),
            cw20_token_2.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEPOSIT, cw20_token_1.clone().into_coin(10_000_000u128))
        .unwrap();
    let market_1 = suite.query_market(cw20_token_1.clone()).unwrap().market;

    assert_eq!(
        suite
            .query_cw20_balance(market_1.as_str(), cw20_token_1.denom())
            .unwrap(),
        10_000_000u128,
        "expected 10_000_000 from deposit of DEPOSIT"
    );

    suite
        .deposit_tokens_on_market(DEPOSIT_2, cw20_token_2.clone().into_coin(10_000_000u128))
        .unwrap();
    let market_2 = suite.query_market(cw20_token_2.clone()).unwrap().market;

    assert_eq!(
        suite
            .query_cw20_balance(market_2.as_str(), cw20_token_2.denom())
            .unwrap(),
        10_000_000u128,
        "expected 10_000_000 from deposit of DEPOSIT_2"
    );

    // User creates a credit line through collateral
    suite
        .deposit_tokens_on_market(ACTOR, cw20_token_1.clone().into_coin(1_000_000u128))
        .unwrap();
    assert_eq!(
        suite
            .query_cw20_balance(market_1.as_str(), cw20_token_1.denom())
            .unwrap(),
        11_000_000u128,
        "expected 10_000_000 from deposit of DEPOSIT + 1_000_000 from ACTOR"
    );

    // User goes into debt, but is still liquid
    suite
        .borrow_tokens_from_market(ACTOR, cw20_token_2.clone().into_coin(1_000_000u128))
        .unwrap();
    assert_eq!(
        suite
            .query_cw20_balance(market_2.as_str(), cw20_token_2.denom())
            .unwrap(),
        9_000_000u128,
        "expected 10_000_000 from deposit of DEPOSIT_2 - 1_000_000 from the borrow of ACTOR"
    );

    let total_credit_line = suite.query_total_credit_line(ACTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 1_000_000 cw20_1 deposited * 2.0 oracle's price
            collateral: Uint128::new(2_000_000),
            // 1000_000 cw20_1 collateral * 2.0 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(1_000_000),
            borrow_limit: Uint128::new(1_000_000),
            // 1_000_000 cw20_2 borrowed * 0.5 oracle's price
            debt: Uint128::new(500_000)
        }
        .make_response(suite.common_token().clone())
    );

    suite
        .repay_with_collateral(
            ACTOR,
            cw20_token_1.clone().into_coin(1_000_000u128),
            cw20_token_2.clone().into_coin(1_000_000u128),
        )
        .unwrap();

    // TODO: is it ok?
    assert_approx_eq!(
        suite
            .query_cw20_balance(market_1.as_str(), cw20_token_1.denom())
            .unwrap(),
        10_750_000u128,
        Decimal::percent(1),
        // "expected approx 11_000_000 cw20_1 - 1_000_000 cw20_2 * 0.5 * 0.5
    );

    assert_eq!(
        suite
            .query_cw20_balance(market_2.as_str(), cw20_token_2.denom())
            .unwrap(),
        10_000_000u128,
        "expected original 10_000_000 since ACTOR debt repaid"
    );

    let total_credit_line = suite.query_total_credit_line(ACTOR).unwrap();
    suite.reset_pools().unwrap();

    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(1_346_662),
            credit_line: Uint128::new(673_331),
            borrow_limit: Uint128::new(673_331),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn on_two_markets_mixed() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(DEPOSIT, &[native_token.clone().into_coin(10_000_000u128)])
        .with_funds(ACTOR, &[native_token.clone().into_coin(5_000_000u128)])
        .with_initial_cw20(cw20_token.denom(), (DEPOSIT_2, 10_000_000))
        .with_pool(
            1,
            (
                common_token.clone().into_coin(20_000_000u128),
                native_token.clone().into_coin(10_000_000u128),
            ), // 2.0
        ) // 0.5
        .build();

    // recover the cw20 address
    let cw20_token = suite
        .starting_cw20
        .get(&cw20_token.denom())
        .unwrap()
        .clone();

    // create cw20 tokens pools
    suite
        .set_pool(&[(
            2,
            (
                common_token.into_coin(5_000_000u128),
                cw20_token.clone().into_coin(10_000_000u128),
            ),
        )])
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token.denom(),
            cw20_token.clone(),
            None,
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEPOSIT, native_token.clone().into_coin(10_000_000u128))
        .unwrap();
    suite
        .deposit_tokens_on_market(DEPOSIT_2, cw20_token.clone().into_coin(10_000_000u128))
        .unwrap();

    // User creates a credit line through collateral
    suite
        .deposit_tokens_on_market(ACTOR, native_token.clone().into_coin(1_000_000u128))
        .unwrap();
    // User goes into debt, but is still liquid
    suite
        .borrow_tokens_from_market(ACTOR, cw20_token.clone().into_coin(1_000_000u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(ACTOR).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 1_000_000 OSMO deposited * 2.0 oracle's price
            collateral: Uint128::new(2_000_000),
            // 1000_000 OSMO collateral * 2.0 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(1_000_000),
            borrow_limit: Uint128::new(1_000_000),
            // 1_000_000 ETH borrowed * 0.5 oracle's price
            debt: Uint128::new(500_000)
        }
        .make_response(suite.common_token().clone())
    );

    suite
        .repay_with_collateral(
            ACTOR,
            native_token.into_coin(1_000_000u128),
            cw20_token.into_coin(1_000_000u128),
        )
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(ACTOR).unwrap();
    suite.reset_pools().unwrap();

    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(1_346_662),
            credit_line: Uint128::new(673_331),
            borrow_limit: Uint128::new(673_331),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn borrow_limit() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_borrow_limit_ratio(Decimal::percent(90))
        .with_funds(
            LIQUIDATOR,
            &[native_token.clone().into_coin(10_000_000u128)],
        )
        .with_funds(DEPOSIT, &[native_token.clone().into_coin(10_000_000u128)])
        .with_pool(
            1,
            (
                common_token.into_coin(10_000_000u128),
                native_token.clone().into_coin(10_000_000u128),
            ),
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            Some(Decimal::percent(80)),
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEPOSIT, native_token.clone().into_coin(10_000_000u128))
        .unwrap();

    // should fail because of borrow limit
    let err = suite
        .borrow_tokens_from_market(DEPOSIT, native_token.clone().into_coin(8_000_000u128))
        .unwrap_err();
    assert_eq!(
        isotonic_market::ContractError::CannotBorrow {
            amount: 8_000_000u128.into(),
            account: DEPOSIT.to_owned(),
        },
        err.downcast().unwrap()
    );

    // we should be able to borrow up to 90% * 80% * 10_000_000 = 7_200_000
    // borrow first part
    suite
        .borrow_tokens_from_market(DEPOSIT, native_token.clone().into_coin(7_000_000u128))
        .unwrap();

    // borrow second part
    suite
        .borrow_tokens_from_market(DEPOSIT, native_token.clone().into_coin(200_000u128))
        .unwrap();

    // any more should fail
    let err = suite
        .borrow_tokens_from_market(DEPOSIT, native_token.clone().into_coin(100_000u128))
        .unwrap_err();
    assert_eq!(
        isotonic_market::ContractError::CannotBorrow {
            amount: 100_000u128.into(),
            account: DEPOSIT.to_owned(),
        },
        err.downcast().unwrap()
    );

    // withdraw also changes the debt / collateral ratio, so we should also not be able to withdraw anymore
    let err = suite
        .withdraw_tokens_from_market(DEPOSIT, native_token.clone().into_coin(1u128))
        .unwrap_err();
    assert_eq!(
        isotonic_market::ContractError::CannotWithdraw {
            amount: 1u128.into(),
            account: DEPOSIT.to_owned(),
        },
        err.downcast().unwrap()
    );

    // make sure deposit cannot be liquidated
    let err = suite
        .liquidate(
            LIQUIDATOR,
            DEPOSIT,
            &[native_token.clone().into_coin(100u128).try_into().unwrap()],
            native_token,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::LiquidationNotAllowed {},
        err.downcast().unwrap()
    );
}

#[test]
fn borrow_limit_cw20() {
    let common_token = Token::Native(COMMON.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_borrow_limit_ratio(Decimal::percent(90))
        .with_initial_cw20(cw20_token.denom(), (LIQUIDATOR, 10_000_000))
        .with_initial_cw20(cw20_token.denom(), (DEPOSIT, 10_000_000))
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
                common_token.into_coin(10_000_000u128),
                cw20_token.clone().into_coin(10_000_000u128),
            ),
        )])
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token.denom(),
            cw20_token.clone(),
            Some(Decimal::percent(80)),
            (Decimal::zero(), Decimal::zero()),
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(DEPOSIT, cw20_token.clone().into_coin(10_000_000u128))
        .unwrap();

    // should fail because of borrow limit
    let err = suite
        .borrow_tokens_from_market(DEPOSIT, cw20_token.clone().into_coin(8_000_000u128))
        .unwrap_err();
    assert_eq!(
        isotonic_market::ContractError::CannotBorrow {
            amount: 8_000_000u128.into(),
            account: DEPOSIT.to_owned(),
        },
        err.downcast().unwrap()
    );

    // we should be able to borrow up to 90% * 80% * 10_000_000 = 7_200_000
    // borrow first part
    suite
        .borrow_tokens_from_market(DEPOSIT, cw20_token.clone().into_coin(7_000_000u128))
        .unwrap();

    // borrow second part
    suite
        .borrow_tokens_from_market(DEPOSIT, cw20_token.clone().into_coin(200_000u128))
        .unwrap();

    // any more should fail
    let err = suite
        .borrow_tokens_from_market(DEPOSIT, cw20_token.clone().into_coin(100_000u128))
        .unwrap_err();
    assert_eq!(
        isotonic_market::ContractError::CannotBorrow {
            amount: 100_000u128.into(),
            account: DEPOSIT.to_owned(),
        },
        err.downcast().unwrap()
    );

    // withdraw also changes the debt / collateral ratio, so we should also not be able to withdraw anymore
    let err = suite
        .withdraw_tokens_from_market(DEPOSIT, cw20_token.clone().into_coin(1u128))
        .unwrap_err();
    assert_eq!(
        isotonic_market::ContractError::CannotWithdraw {
            amount: 1u128.into(),
            account: DEPOSIT.to_owned(),
        },
        err.downcast().unwrap()
    );

    // make sure deposit cannot be liquidated
    let err = suite
        .liquidate_with_cw20(
            LIQUIDATOR,
            DEPOSIT,
            cw20_token.clone().into_coin(100u128),
            cw20_token,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::LiquidationNotAllowed {},
        err.downcast().unwrap()
    );
}
