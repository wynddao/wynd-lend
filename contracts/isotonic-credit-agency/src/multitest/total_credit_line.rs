use crate::multitest::suite::{
    BORROWER, BORROWER_2, GOVERNANCE, JUNO, LENDER, LENDER_2, MARKET_TOKEN, OSMO, WYND,
};

use super::suite::{SuiteBuilder, COMMON};

use cosmwasm_std::Uint128;
use utils::{credit_line::CreditLineValues, token::Token};

#[test]
fn lender_on_one_market() {
    let common_token = Token::Native(COMMON.to_owned());
    let market_token = Token::Native(MARKET_TOKEN.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LENDER, &[market_token.clone().into_coin(1000u128)])
        // Sets sell/buy rate between market denom/common denom as 2.0,
        // which means selling 1000 market denom will result in 2000 common denom
        .with_pool(
            1,
            (
                common_token.into_coin(200u128),
                market_token.clone().into_coin(100u128),
            ),
        )
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &market_token.denom(),
            market_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    suite
        .deposit_tokens_on_market(LENDER, market_token.into_coin(1000u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(LENDER).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            collateral: Uint128::new(2000),
            // 1000 collateral * 2.0 oracle's price * 0.5 collateral_ratio (default in crate_market_quick)
            credit_line: Uint128::new(1000),
            borrow_limit: Uint128::new(1000),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn lender_on_three_markets() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(OSMO.to_owned());

    let cw20_token_1 = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(
            LENDER,
            &[
                native_token_1.clone().into_coin(1_000u128),
                native_token_2.clone().into_coin(500u128),
            ],
        )
        .with_initial_cw20(cw20_token_1.denom(), (LENDER, 500))
        // Sets sell/buy rate between market_1 token/common denom as 2.0,
        // selling 1000 native_token_1 tokens gives 2000 common tokens.
        .with_pool(
            1,
            (
                common_token.clone().into_coin(200u128),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        // here - selling 500 native_token_2 gives 250 common
        .with_pool(
            2,
            (
                common_token.clone().into_coin(50u128),
                native_token_2.clone().into_coin(100u128),
            ),
        )
        .build();

    let cw20_token_1 = suite
        .starting_cw20
        .get(&cw20_token_1.denom())
        .unwrap()
        .clone();
    // Create cw20 tokens pools.
    // here - selling 7 cw20_1 tokens gives 7000 common tokens.
    suite
        .set_pool(&[(
            3,
            (
                common_token.into_coin(100_000u128),
                cw20_token_1.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    // Create 2 native markets and 1 cw20 markets.
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
            &cw20_token_1.denom(),
            cw20_token_1.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // Deposit tokens on the three markets
    suite
        .deposit_tokens_on_market(LENDER, native_token_1.into_coin(1000u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(LENDER, native_token_2.into_coin(500u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(LENDER, cw20_token_1.into_coin(7u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(LENDER).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 1000 deposited * 2.0 oracle's price + 500 deposited * 0.5 oracle's price
            //   + 7 * 1000.0 oracle's price
            collateral: Uint128::new(9250),
            // 1000 collateral * 2.0 oracle's price * 0.5 default collateral_ratio
            //   + 500 collateral * 0.5 oracle's price * 0.5 default collateral_ratio
            //   + 7 collateral * 1000.0 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(4625),
            borrow_limit: Uint128::new(4625),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn lender_on_two_markets_with_two_borrowers() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());

    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LENDER, &[native_token.clone().into_coin(100u128)])
        .with_funds(BORROWER, &[native_token.clone().into_coin(1_000u128)])
        .with_initial_cw20(cw20_token.denom(), (LENDER, 500))
        .with_initial_cw20(cw20_token.denom(), (BORROWER_2, 1_500))
        // Sets sell/buy rate between native token /common token as 2.0,
        // selling 1000 native tokens gives 2000 common tokens
        .with_pool(
            1,
            (
                common_token.clone().into_coin(200u128),
                native_token.clone().into_coin(100u128),
            ),
        )
        .build();

    let cw20_token = suite
        .starting_cw20
        .get(&cw20_token.denom())
        .unwrap()
        .clone();

    // Create cw20 tokens pools.
    // here - selling 500 cw20 tokens gives 250 common tokens.
    suite
        .set_pool(&[(
            3,
            (
                common_token.into_coin(50u128),
                cw20_token.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token.denom(),
            cw20_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // LENDER deposits all his tokens.
    suite
        .deposit_tokens_on_market(LENDER, native_token.clone().into_coin(100u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(LENDER, cw20_token.clone().into_coin(500u128))
        .unwrap();

    // First borrower deposits 1000 owned tokens and then borrows
    suite
        .deposit_tokens_on_market(BORROWER, native_token.clone().into_coin(1000u128))
        .unwrap();

    suite
        .borrow_tokens_from_market(BORROWER, cw20_token.clone().into_coin(500u128))
        .unwrap();

    // Second borrower deposits 1500 owned tokens and then borrows
    suite
        .deposit_tokens_on_market(BORROWER_2, cw20_token.into_coin(1_500u128))
        .unwrap();

    suite
        .borrow_tokens_from_market(BORROWER_2, native_token.into_coin(100u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(LENDER).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 100 deposited * 2.0 oracle's price + 500 deposited * 0.5 oracle's price
            collateral: Uint128::new(450),
            // 100 collateral * 2.0 oracle's price * 0.5 default collateral_ratio
            //   + 500 collateral * 0.5 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(225),
            borrow_limit: Uint128::new(225),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );

    let total_credit_line = suite.query_total_credit_line(BORROWER).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 1000 deposited * 2.0 oracle's price
            collateral: Uint128::new(2000),
            // 1000 collateral * 2.0 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(1000),
            borrow_limit: Uint128::new(1000),
            // 500 borrowed * 0.5 oracle's price (second denom)
            debt: Uint128::new(250)
        }
        .make_response(suite.common_token().clone())
    );

    let total_credit_line = suite.query_total_credit_line(BORROWER_2).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 1500 deposited * 0.5 oracle's price
            collateral: Uint128::new(750),
            // 1500 collateral * 0.5 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(375),
            borrow_limit: Uint128::new(375),
            // 100 borrowed * 2.0 oracle's price (first denom)
            debt: Uint128::new(200)
        }
        .make_response(suite.common_token().clone())
    );
}

#[test]
fn two_lenders_with_borrower_on_two_markets() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());

    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(LENDER, &[native_token.clone().into_coin(500u128)])
        .with_funds(BORROWER, &[native_token.clone().into_coin(3_000u128)])
        .with_initial_cw20(cw20_token.denom(), (LENDER_2, 300))
        // Sets sell/buy rate between native token /common token as 2.0,
        // selling 1000 native tokens gives 2000 common tokens
        .with_pool(
            1,
            (
                common_token.clone().into_coin(150u128),
                native_token.clone().into_coin(100u128),
            ),
        )
        .build();

    let cw20_token = suite
        .starting_cw20
        .get(&cw20_token.denom())
        .unwrap()
        .clone();

    // Create cw20 tokens pools.
    // here - selling 500 cw20 tokens gives 250 common tokens.
    suite
        .set_pool(&[(
            3,
            (
                common_token.into_coin(50u128),
                cw20_token.clone().into_coin(100u128),
            ),
        )])
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &native_token.denom(),
            native_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token.denom(),
            cw20_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // Lenders deposits all their tokens.
    suite
        .deposit_tokens_on_market(LENDER, native_token.clone().into_coin(500u128))
        .unwrap();
    suite
        .deposit_tokens_on_market(LENDER_2, cw20_token.clone().into_coin(300u128))
        .unwrap();

    // BORROWER deposits his tokens on first market, then borrows from first and second market
    suite
        .deposit_tokens_on_market(BORROWER, native_token.clone().into_coin(3_000u128))
        .unwrap();

    suite
        .borrow_tokens_from_market(BORROWER, native_token.into_coin(500u128))
        .unwrap();
    suite
        .borrow_tokens_from_market(BORROWER, cw20_token.into_coin(300u128))
        .unwrap();

    let total_credit_line = suite.query_total_credit_line(LENDER).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 500 deposited * 1.5 oracle's price
            collateral: Uint128::new(750),
            // 500 collateral * 1.5 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(375),
            borrow_limit: Uint128::new(375),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );

    let total_credit_line = suite.query_total_credit_line(LENDER_2).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 300 deposited * 0.5 oracle's price
            collateral: Uint128::new(150),
            // 300 collateral * 0.5 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(75),
            borrow_limit: Uint128::new(75),
            debt: Uint128::zero()
        }
        .make_response(suite.common_token().clone())
    );

    let total_credit_line = suite.query_total_credit_line(BORROWER).unwrap();
    assert_eq!(
        total_credit_line,
        CreditLineValues {
            // 3000 deposited * 1.5 oracle's price
            collateral: Uint128::new(4500),
            // 3000 collateral * 1.5 oracle's price * 0.5 default collateral_ratio
            credit_line: Uint128::new(2250),
            borrow_limit: Uint128::new(2250),
            // 500 borrowed * 1.5 oracle's price + 300 borrowed * 0.5 oracle's price
            debt: Uint128::new(900)
        }
        .make_response(suite.common_token().clone())
    );
}
