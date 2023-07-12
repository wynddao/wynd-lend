use cosmwasm_std::Addr;

use super::suite::SuiteBuilder;
use crate::{
    error::ContractError,
    multitest::suite::{ACTOR, ACTOR_2, ATOM, COMMON, DAODAO, GOVERNANCE, JUNO, OSMO, WYND},
};

use utils::{coin::Coin, token::Token};

#[test]
fn enter_market() {
    // This test does not test full flow in which market would be entered on first market operation,
    // it just tests if the market contract can properly introduce account to it. For this reason,
    // we don't need to test with instantiated cw20 tokens.

    let native_token = Token::Native(JUNO.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    // Works with native.
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

    let market = suite.query_market(native_token).unwrap().market;

    assert!(!suite.is_on_market(ACTOR, market.as_str()).unwrap());

    let markets = suite.list_all_entered_markets(ACTOR).unwrap();
    assert!(markets.is_empty());

    suite.enter_market(market.as_str(), ACTOR).unwrap();

    assert!(suite.is_on_market(ACTOR, market.as_str()).unwrap());

    let markets = suite.list_all_entered_markets(ACTOR).unwrap();
    assert_eq!(markets, vec![market.clone()]);

    // works with cw20.
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

    let market = suite.query_market(cw20_token).unwrap().market;

    assert!(!suite.is_on_market(ACTOR, market.as_str()).unwrap());

    suite.enter_market(market.as_str(), ACTOR).unwrap();

    assert!(suite.is_on_market(ACTOR, market.as_str()).unwrap());

    let markets = suite.list_all_entered_markets(ACTOR).unwrap();
    assert_eq!(markets.len(), 2);
}

#[test]
fn enter_market_by_deposit() {
    let native_token = Token::Native(JUNO.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let native_coin = native_token.clone().into_coin(500u128);

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(ACTOR, &[native_coin])
        .with_initial_cw20(cw20_token.denom(), (ACTOR, 1_000))
        .build();

    // Can create a market with native token.
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

    // Can deposit native coins in a native market.
    suite
        .deposit_tokens_on_market(ACTOR, native_token.clone().into_coin(500u128))
        .unwrap();

    // Assert that actor is entered in the native market.
    let market_1 = suite.query_market(native_token).unwrap().market;
    assert!(suite.is_on_market(ACTOR, market_1.as_str()).unwrap());

    // Assert that the list of markets contains only one market equal to the native one.
    let markets = suite.list_all_entered_markets(ACTOR).unwrap();
    assert_eq!(markets, vec![market_1.clone()]);

    // Can create market with cw20 tokens.
    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token.denom(),
            suite
                .starting_cw20
                .get(&cw20_token.denom())
                .unwrap()
                .clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // Can deposit tokens in cw20 market routed through the cw20 contract.
    suite
        .deposit_tokens_on_market(
            ACTOR,
            suite
                .starting_cw20
                .get(&cw20_token.denom())
                .unwrap()
                .clone()
                .into_coin(500u128),
        )
        .unwrap();

    // Assert that actor is entered in the cw20 market.
    let market_2 = suite
        .query_market(
            suite
                .starting_cw20
                .get(&cw20_token.denom())
                .unwrap()
                .clone(),
        )
        .unwrap()
        .market;
    assert!(suite.is_on_market(ACTOR, market_2.as_str()).unwrap());

    // Assert that the list of markets contains both the native and the cw20 market.
    let markets = suite.list_all_entered_markets(ACTOR).unwrap();
    assert_eq!(markets, vec![market_1, market_2]);
}

#[test]
fn enter_market_by_borrow() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(OSMO.to_owned());

    let cw20_token_1 = Token::Cw20(WYND.to_owned());
    let cw20_token_2 = Token::Cw20(DAODAO.to_owned());

    let common_coin = common_token.into_coin(100u128);
    let native_coin_1 = native_token_1.clone().into_coin(500u128);
    let native_coin_2 = native_token_2.clone().into_coin(500u128);

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(ACTOR, &[native_coin_1])
        .with_funds(ACTOR_2, &[native_coin_2])
        .with_initial_cw20(cw20_token_1.denom(), (ACTOR, 500))
        .with_initial_cw20(cw20_token_2.denom(), (ACTOR_2, 500))
        .with_pool(
            1,
            (
                common_coin.clone(),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_coin.clone(),
                native_token_2.clone().into_coin(100u128),
            ),
        )
        .build();

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

    // Create cw20 tokens pools.
    suite
        .set_pool(&[(
            3,
            (common_coin.clone(), cw20_token_1.clone().into_coin(100u128)),
        )])
        .unwrap();
    suite
        .set_pool(&[(4, (common_coin, cw20_token_2.clone().into_coin(100u128)))])
        .unwrap();

    // Create 2 native markets and 2 cw20 markets.
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

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token_2.denom(),
            cw20_token_2.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // Actor 1 deposits collateral tokens on market 1 and 3.
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(500u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(ACTOR, cw20_token_1.clone().into_coin(500u128))
        .unwrap();

    // Actor 2 deposit collateral tokens on market 2 and 4.
    suite
        .deposit_tokens_on_market(ACTOR_2, native_token_2.clone().into_coin(500u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(ACTOR_2, cw20_token_2.clone().into_coin(500u128))
        .unwrap();

    // Actor 1 can borrow native tokens from market 2
    suite
        .borrow_tokens_from_market(ACTOR, native_token_2.clone().into_coin(200u128))
        .unwrap();

    let market_1 = suite.query_market(native_token_1).unwrap().market;
    let market_2 = suite.query_market(native_token_2).unwrap().market;
    let market_3 = suite.query_market(cw20_token_1).unwrap().market;

    assert!(suite.is_on_market(ACTOR, market_2.as_str()).unwrap());

    let mut markets = suite.list_all_entered_markets(ACTOR).unwrap();
    markets.sort();
    assert_eq!(
        markets,
        vec![market_1.clone(), market_2.clone(), market_3.clone()]
    );

    // Actor 1 can borrow cw20 tokens from market 4
    suite
        .borrow_tokens_from_market(ACTOR, cw20_token_2.clone().into_coin(200u128))
        .unwrap();

    let market_4 = suite.query_market(cw20_token_2).unwrap().market;

    assert!(suite.is_on_market(ACTOR, market_4.as_str()).unwrap());

    let mut markets = suite.list_all_entered_markets(ACTOR).unwrap();
    markets.sort();
    assert_eq!(markets, vec![market_4, market_1, market_2, market_3]);
}

#[test]
fn exit_market() {
    let native_token = Token::Native(JUNO.to_owned());

    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    // Can enter and exit from a native market
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

    let market = suite.query_market(native_token).unwrap().market;
    suite.enter_market(market.as_str(), ACTOR).unwrap();

    suite.exit_market(ACTOR, market.as_str()).unwrap();

    assert!(!suite.is_on_market(ACTOR, market.as_str()).unwrap());
    assert!(suite.list_all_entered_markets(ACTOR).unwrap().is_empty());

    // Can enter and exit from a cw20 market
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

    let market = suite.query_market(cw20_token).unwrap().market;
    suite.enter_market(market.as_str(), ACTOR).unwrap();

    suite.exit_market(ACTOR, market.as_str()).unwrap();

    assert!(!suite.is_on_market(ACTOR, market.as_str()).unwrap());
    assert!(suite.list_all_entered_markets(ACTOR).unwrap().is_empty());
}

#[test]
fn cant_exit_market_not_being_part_of() {
    let native_token = Token::Native(JUNO.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    //
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

    let market = suite.query_market(native_token).unwrap().market;

    let err = suite.exit_market(ACTOR, market.as_str()).unwrap_err();

    assert_eq!(
        ContractError::NotOnMarket {
            address: Addr::unchecked(ACTOR),
            market: market.clone()
        },
        err.downcast().unwrap()
    );

    assert!(!suite.is_on_market(ACTOR, market.as_str()).unwrap());
    assert!(suite.list_all_entered_markets(ACTOR).unwrap().is_empty());
}

#[test]
fn cant_exit_market_with_borrowed_tokens() {
    // Use case:
    // 1. actor_1 deposits tokens on market_1 to have some collateral
    // 2. actor_2 deposits tokens on market_2 so there is something to borrow
    // 3. actor_1 borrows tokens from market_2
    // 4. actor_1 tries to exit market_2, which fails as he has tokens borrowed there.
    // This test is implemented both for cw20 and native.
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(OSMO.to_owned());

    let cw20_token_1 = Token::Cw20(WYND.to_owned());
    let cw20_token_2 = Token::Cw20(DAODAO.to_owned());

    let common_coin = common_token.into_coin(100u128);
    let native_coin_1 = native_token_1.clone().into_coin(500u128);
    let native_coin_2 = native_token_2.clone().into_coin(500u128);

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(ACTOR, &[native_coin_1])
        .with_funds(ACTOR_2, &[native_coin_2])
        .with_initial_cw20(cw20_token_1.denom(), (ACTOR, 500))
        .with_initial_cw20(cw20_token_2.denom(), (ACTOR_2, 500))
        .with_pool(
            1,
            (
                common_coin.clone(),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_coin.clone(),
                native_token_2.clone().into_coin(100u128),
            ),
        )
        .build();

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

    // Create cw20 tokens pools.
    suite
        .set_pool(&[(
            3,
            (common_coin.clone(), cw20_token_1.clone().into_coin(100u128)),
        )])
        .unwrap();
    suite
        .set_pool(&[(4, (common_coin, cw20_token_2.clone().into_coin(100u128)))])
        .unwrap();

    // Create 2 native markets and 2 cw20 markets.
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

    suite
        .create_market_quick(
            GOVERNANCE,
            &cw20_token_2.denom(),
            cw20_token_2.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // Actor 1 deposits collateral tokens on market 1 and 3.
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(500u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(ACTOR, cw20_token_1.clone().into_coin(500u128))
        .unwrap();

    // Actor 2 deposit collateral tokens on market 2 and 4.
    suite
        .deposit_tokens_on_market(ACTOR_2, native_token_2.clone().into_coin(500u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(ACTOR_2, cw20_token_2.clone().into_coin(500u128))
        .unwrap();

    // Actor 1 can borrow tokens from market 2
    suite
        .borrow_tokens_from_market(ACTOR, native_token_2.clone().into_coin(200u128))
        .unwrap();

    let market_1 = suite.query_market(native_token_1).unwrap().market;
    let market_2 = suite.query_market(native_token_2).unwrap().market;
    let market_3 = suite.query_market(cw20_token_1).unwrap().market;

    // actor_1 cannot exit since it still have borrowed tokens on native market_2.
    let err = suite.exit_market(ACTOR, market_2.as_str()).unwrap_err();
    assert_eq!(
        ContractError::DebtOnMarket {
            address: Addr::unchecked(ACTOR),
            market: market_2.clone(),
            debt: Coin::new(200, suite.common_token().clone()),
        },
        err.downcast().unwrap()
    );

    let mut markets = suite.list_all_entered_markets(ACTOR).unwrap();
    markets.sort();

    assert_eq!(
        markets,
        vec![market_1.clone(), market_2.clone(), market_3.clone()]
    );

    suite
        .borrow_tokens_from_market(ACTOR, cw20_token_2.clone().into_coin(200u128))
        .unwrap();

    let market_4 = suite.query_market(cw20_token_2).unwrap().market;

    // actor_1 cannot exit since it still have borrowed tokens on cw20 market_4.
    let err = suite.exit_market(ACTOR, market_4.as_str()).unwrap_err();
    assert_eq!(
        ContractError::DebtOnMarket {
            address: Addr::unchecked(ACTOR),
            market: market_4.clone(),
            debt: Coin::new(200, suite.common_token().clone()),
        },
        err.downcast().unwrap()
    );

    assert!(suite.is_on_market(ACTOR, market_4.as_str()).unwrap());

    let mut markets = suite.list_all_entered_markets(ACTOR).unwrap();
    markets.sort();
    assert_eq!(markets, vec![market_4, market_1, market_2, market_3]);
}

#[test]
fn cant_exit_market_with_not_enough_liquidity() {
    // Use case:
    // 1. Actor1 deposits tokens on denom1 to have some collateral
    // 2. Actor2 deposits tokens on denom2 so there is something to borrow
    // 3. Actor1 borrows denom2 tokens
    // 4. Actor1 tries to exit denom1 market, which fails as after that he would not have enough
    //    collateral to cover denom2 debta
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(OSMO.to_owned());

    let cw20_token_1 = Token::Cw20(WYND.to_owned());

    let common_coin = common_token.into_coin(100u128);
    let native_coin_1 = native_token_1.clone().into_coin(500u128);
    let native_coin_2 = native_token_2.clone().into_coin(500u128);

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(ACTOR, &[native_coin_1])
        .with_funds(ACTOR_2, &[native_coin_2])
        .with_initial_cw20(cw20_token_1.denom(), (ACTOR, 500))
        .with_pool(
            1,
            (
                common_coin.clone(),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_coin.clone(),
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
    suite
        .set_pool(&[(3, (common_coin, cw20_token_1.clone().into_coin(100u128)))])
        .unwrap();

    // Create 2 native markets and 2 cw20 markets.
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

    // Actor 1 deposits collateral tokens on market 1.
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(500u128))
        .unwrap();

    // Actor 2 deposit collateral tokens on market 2 and 4.
    suite
        .deposit_tokens_on_market(ACTOR_2, native_token_2.clone().into_coin(500u128))
        .unwrap();

    // Actor 1  borrow tokens from market 2
    suite
        .borrow_tokens_from_market(ACTOR, native_token_2.clone().into_coin(200u128))
        .unwrap();

    let market_1 = suite.query_market(native_token_1).unwrap().market;
    let market_2 = suite.query_market(native_token_2).unwrap().market;
    let market_3 = suite.query_market(cw20_token_1.clone()).unwrap().market;

    // actor1 needs tokens from market1 to have enough liquidity for market2 debt
    let err = suite.exit_market(ACTOR, market_1.as_str()).unwrap_err();
    assert_eq!(
        ContractError::NotEnoughCollat {
            credit_line: 0u128.into(),
            collateral: 0u128.into(),
            debt: 200u128.into(),
        },
        err.downcast().unwrap()
    );

    suite
        .deposit_tokens_on_market(ACTOR, cw20_token_1.into_coin(500u128))
        .unwrap();

    let mut markets = suite.list_all_entered_markets(ACTOR).unwrap();
    markets.sort();

    assert_eq!(markets, vec![market_1, market_2, market_3]);
}

#[test]
fn exit_market_with_ctokens() {
    // Use case:
    // 1. actor_1 deposits tokens on market_1 to have some collateral
    // 2. actor_1 deposits tokens on market_3 just because he can
    // 3. actor_2 deposits tokens on market_3 so there is something to borrow
    // 4. actor_1 borrows tokens from market_2
    // 5. actor_1 can exit market_3 since he covers market_2 debt with market_1 ctokens
    let common_token = Token::Native(COMMON.to_owned());
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(OSMO.to_owned());
    let native_token_3 = Token::Native(ATOM.to_owned());

    let cw20_token_1 = Token::Cw20(WYND.to_owned());

    let common_coin = common_token.into_coin(100u128);
    let native_coin_1 = native_token_1.clone().into_coin(500u128);
    let native_coin_2 = native_token_2.clone().into_coin(500u128);
    let native_coin_3 = native_token_3.clone().into_coin(500u128);

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_funds(ACTOR, &[native_coin_1, native_coin_3])
        .with_funds(ACTOR_2, &[native_coin_2])
        .with_initial_cw20(cw20_token_1.denom(), (ACTOR, 500))
        .with_pool(
            1,
            (
                common_coin.clone(),
                native_token_1.clone().into_coin(100u128),
            ),
        )
        .with_pool(
            2,
            (
                common_coin.clone(),
                native_token_2.clone().into_coin(100u128),
            ),
        )
        .with_pool(3, (common_coin, native_token_3.clone().into_coin(100u128)))
        .build();

    // Create 3 native markets.
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

    // Actor 1 deposits collateral tokens on market 1 and 3.
    suite
        .deposit_tokens_on_market(ACTOR, native_token_1.clone().into_coin(500u128))
        .unwrap();

    suite
        .deposit_tokens_on_market(ACTOR, native_token_3.clone().into_coin(200u128))
        .unwrap();

    // Actor 2 deposits collateral tokens on market 2.
    suite
        .deposit_tokens_on_market(ACTOR_2, native_token_2.clone().into_coin(500u128))
        .unwrap();

    // Actor 1  borrow tokens from market 2
    suite
        .borrow_tokens_from_market(ACTOR, native_token_2.clone().into_coin(200u128))
        .unwrap();

    let market_1 = suite.query_market(native_token_1).unwrap().market;
    let market_2 = suite.query_market(native_token_2).unwrap().market;
    let market_3 = suite.query_market(native_token_3).unwrap().market;

    suite.exit_market(ACTOR, market_3.as_str()).unwrap();

    let mut markets = suite.list_all_entered_markets(ACTOR).unwrap();
    markets.sort();

    assert_eq!(markets, vec![market_1, market_2]);
}
