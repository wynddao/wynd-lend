use super::suite::{contract_market, SuiteBuilder};
use crate::{
    error::ContractError,
    multitest::suite::{COMMON, GOVERNANCE, JUNO, MARKET_TOKEN, OWNER, WYND},
};

use isotonic_market::msg::MigrateMsg as MarketMigrateMsg;
use utils::token::Token;

#[test]
fn adjust_market_id() {
    let mut suite = SuiteBuilder::new().build();

    suite.sudo_adjust_market_id(30).unwrap();
    assert_eq!(30, suite.query_config().unwrap().isotonic_market_id);
}

#[test]
fn adjust_token_id() {
    let mut suite = SuiteBuilder::new().build();

    suite.sudo_adjust_token_id(30).unwrap();
    assert_eq!(30, suite.query_config().unwrap().isotonic_token_id);
}

#[test]
fn adjust_common_token() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_common_token(common_token)
        .build();

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

    let new_common = Token::Native(MARKET_TOKEN.to_owned());
    suite.sudo_adjust_common_token(new_common.clone()).unwrap();
    assert_eq!(new_common, suite.query_config().unwrap().common_token);
    assert_eq!(
        new_common,
        suite
            .query_market_config(native_token)
            .unwrap()
            .common_token
    );
    assert_eq!(
        new_common,
        suite.query_market_config(cw20_token).unwrap().common_token
    );
}

#[test]
fn migrate_market_native() {
    let common_token = Token::Native(COMMON.to_owned());
    let native_token = Token::Native(JUNO.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_common_token(common_token)
        .build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &native_token.denom()),
            native_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // First code_id is cw20, second is oracle and then:
    assert_eq!(
        3,
        suite.query_contract_code_id(native_token.clone()).unwrap()
    );
    assert_eq!(
        4,
        suite
            .query_market_config(native_token.clone())
            .unwrap()
            .token_id
    );

    let new_market_id = suite.app().store_code(contract_market());
    assert_ne!(
        new_market_id,
        suite.query_contract_code_id(native_token.clone()).unwrap()
    );

    suite.sudo_adjust_market_id(new_market_id).unwrap();

    let market = suite.query_market(native_token.clone()).unwrap();
    suite
        .sudo_migrate_market(
            market.market.as_str(),
            MarketMigrateMsg {
                isotonic_token_id: Some(50),
            },
        )
        .unwrap();

    assert_eq!(
        new_market_id,
        suite.query_contract_code_id(native_token.clone()).unwrap()
    );
    assert_eq!(
        50,
        suite.query_market_config(native_token).unwrap().token_id
    );
}

#[test]
fn migrate_market_cw20() {
    let common_token = Token::Native(COMMON.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_initial_cw20(cw20_token.denom(), (OWNER, 1))
        .with_common_token(common_token)
        .build();

    let cw20_token = suite
        .starting_cw20
        .get(&cw20_token.denom())
        .unwrap()
        .clone();

    suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &cw20_token.denom()),
            cw20_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    // First code_id is cw20, second is oracle and then:
    assert_eq!(3, suite.query_contract_code_id(cw20_token.clone()).unwrap());
    assert_eq!(
        4,
        suite
            .query_market_config(cw20_token.clone())
            .unwrap()
            .token_id
    );

    let new_market_id = suite.app().store_code(contract_market());
    assert_ne!(
        new_market_id,
        suite.query_contract_code_id(cw20_token.clone()).unwrap()
    );

    suite.sudo_adjust_market_id(new_market_id).unwrap();

    let market = suite.query_market(cw20_token.clone()).unwrap();
    suite
        .sudo_migrate_market(
            market.market.as_str(),
            MarketMigrateMsg {
                isotonic_token_id: Some(50),
            },
        )
        .unwrap();

    assert_eq!(
        new_market_id,
        suite.query_contract_code_id(cw20_token.clone()).unwrap()
    );
    assert_eq!(50, suite.query_market_config(cw20_token).unwrap().token_id);
}

#[test]
fn migrate_non_existing_market() {
    let common_token = Token::Native(COMMON.to_owned());

    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_common_token(common_token)
        .build();

    let err = suite
        .sudo_migrate_market(
            WYND,
            MarketMigrateMsg {
                isotonic_token_id: None,
            },
        )
        .unwrap_err();

    assert_eq!(
        ContractError::MarketSearchError {
            market: WYND.to_owned(),
        },
        err.downcast().unwrap()
    );
}
