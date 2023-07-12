use cosmwasm_std::Uint128;
use utils::{
    coin::{coin_cw20, coin_native},
    token::Token,
};
use wyndex::factory::PairType;

use super::suite::{SuiteBuilder, COMMON};
use crate::{
    error::ContractError,
    multitest::suite::{MARKET_TOKEN, OWNER, USDC, USER, WYND},
};

#[test]
fn sender_not_credit_agency() {
    let mut suite = SuiteBuilder::new().build();

    let err = suite
        .swap_withdraw_from(
            "any sender",
            "account",
            Uint128::zero(),
            coin_native(100, "denom"),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::RequiresCreditAgency {},
        err.downcast().unwrap()
    );
}

#[test]
fn two_denoms() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());
    let usdc_token = Token::Native(USDC.to_owned());
    let common_token = Token::Native(COMMON.to_string());

    let mut suite = SuiteBuilder::new()
        .with_market_token(market_token.clone())
        .with_funds(USER, &[market_token.clone().into_coin(5_000_000u128)])
        .build();

    suite.create_pool_with_liquidity_and_twap_price(
        common_token.clone().into_coin(1_000_000_000_000_000u128),
        market_token.clone().into_coin(1_000_000_000_000_000u128),
        PairType::Xyk {},
    );

    suite.create_pool_with_liquidity_and_twap_price(
        common_token.clone().into_coin(1_000_000_000_000_000u128),
        usdc_token.clone().into_coin(1_000_000_000_000_000u128),
        PairType::Xyk {},
    );

    suite.deposit(USER, market_token, 5_000_000u128).unwrap();
    assert_eq!(suite.query_contract_asset_balance().unwrap(), 5_000_000);

    let ca = suite.credit_agency();
    // Buy 4.5M USDC, using maximally 5M MARKET_TOKEN tokens for that
    suite
        .swap_withdraw_from(
            ca,
            USER,
            Uint128::new(5_000_000),
            coin_native(4_500_000, USDC),
        )
        .unwrap();


    suite.advance_seconds(3600);

    let resp = dbg!(suite.query_contract_asset_balance().unwrap());
    dbg!(suite.query_tokens_balance(USER).unwrap());
    dbg!(suite.query_asset_balance(USER, "usdc".to_owned()).unwrap());

    // Excluding swap fees, amount left on contract should be less or equal to 0.5M tokens
    assert!(
        matches!(resp, x if x > 470_000 && x <= 500_000)
    );
}

#[test]
fn sell_limit_lesser_then_required() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());
    let usdc_token = Token::Native(USDC.to_owned());
    let common_token = Token::Native(COMMON.to_string());

    let mut suite = SuiteBuilder::new()
        .with_market_token(market_token.clone())
        .with_funds(USER, &[market_token.clone().into_coin(5_000_000u128)])
        .build();

    suite.create_pool_with_liquidity_and_twap_price(
        common_token.clone().into_coin(1_000_000_000_000_000u128),
        market_token.clone().into_coin(1_000_000_000_000_000u128),
        PairType::Xyk {},
    );

    suite.create_pool_with_liquidity_and_twap_price(
        common_token.clone().into_coin(1_000_000_000_000_000u128),
        usdc_token.clone().into_coin(1_000_000_000_000_000u128),
        PairType::Xyk {},
    );

    suite.deposit(USER, market_token, 5_000_000u128).unwrap();
    assert_eq!(suite.query_contract_asset_balance().unwrap(), 5_000_000);

    let ca = suite.credit_agency();
    // Since price ratio is 1:1, sell limit == buy will fail because of fees
    suite
        .swap_withdraw_from(
            ca,
            USER,
            Uint128::new(4_500_000),
            coin_native(4_500_000, usdc_token.denom()),
        )
        .unwrap_err();
    // TODO: How to assert querier error?
}

#[test]
fn same_denom() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_market_token(market_token.clone())
        .with_funds(USER, &[market_token.clone().into_coin(5_000_000u128)])
        .build();

    suite
        .deposit(USER, market_token.clone(), 5_000_000u128)
        .unwrap();

    let ca = suite.credit_agency();
    suite
        .swap_withdraw_from(
            ca,
            USER,
            Uint128::new(4_500_000),
            coin_native(4_500_000, market_token.denom()),
        )
        .unwrap();

    // Excluding swap fees, amount left on contract should be equal to 0.5M tokens,
    // becase no fees are included
    assert!(matches!(
        suite.query_contract_asset_balance().unwrap(),
        500_000
    ));
}

/*
#[test]
fn buy_common_denom() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());
    let common_token = Token::Native(COMMON.to_string());


    let mut suite = SuiteBuilder::new()
        .with_market_token(market_token.clone())
        .with_funds(USER, &[market_token.clone().into_coin(5_000_000u128)])
        .build();

    suite.create_pool_with_liquidity_and_twap_price(
        common_token.clone().into_coin(1_000_000_000_000_000u128),
        market_token.clone().into_coin(1_000_000_000_000_000u128),
        PairType::Xyk {},
    );

    suite.deposit(USER, market_token, 5_000_000u128).unwrap();

    let ca = suite.credit_agency();
    suite
        .swap_withdraw_from(
            ca,
            USER,
            Uint128::new(5_000_000),
            coin_native(4_500_000, COMMON),
        )
        .unwrap();

    // Excluding swap fees, amount left on contract should be less or equal to 0.5M tokens
    // Similar as in two_denoms testcase, but here estimate goes through only one LP so fee
    // is twice smaller
    assert!(
        matches!(suite.query_contract_asset_balance().unwrap(), x if x > 485_000 && x <= 500_000)
    );
}

#[test]
fn market_uses_common_token() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());
    let common_token = Token::Native(COMMON.to_string());

    let market_coin = market_token.clone().into_coin(100_000_000_000u128);
    let common_coin = common_token.clone().into_coin(100_000_000_000u128);

    let mut suite = SuiteBuilder::new()
        .with_market_token(common_token.clone())
        .with_funds(USER, &[common_token.clone().into_coin(5_000_000u128)])
        .with_pool(1, (common_coin.clone(), market_coin.clone()))
        .with_pool(2, (market_coin, common_coin))
        .build();

    suite.deposit(USER, common_token, 5_000_000u128).unwrap();

    let ca = suite.credit_agency();
    suite
        .swap_withdraw_from(
            ca,
            USER,
            Uint128::new(5_000_000),
            coin_native(4_500_000, market_token.denom()),
        )
        .unwrap();

    // Excluding swap fees, amount left on contract should be less or equal to 0.5M tokens
    // Similar as in buy_common_denom testcase, but here estimate goes through only one LP so fee
    // is twice smaller
    assert!(
        matches!(suite.query_contract_asset_balance().unwrap(), x if x > 485_000 && x <= 500_000)
    );
}
 */
