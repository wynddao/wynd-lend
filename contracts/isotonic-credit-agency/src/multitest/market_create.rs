use super::suite::{SuiteBuilder, DAODAO, GOVERNANCE, JUNO, MARKET_TOKEN, WYND};
use crate::error::ContractError;

use cosmwasm_std::Decimal;
use utils::token::Token;

#[test]
fn market_create_native() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &market_token.denom()),
            market_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();
    suite.assert_market(market_token);
}

#[test]
fn market_create_cw20() {
    let market_token = Token::Cw20(MARKET_TOKEN.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &market_token.denom()),
            market_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();
    suite.assert_market(market_token);
}

#[test]
fn market_create_multiple() {
    let native_token = Token::Native(JUNO.to_owned());
    let cw20_token_1 = Token::Cw20(WYND.to_owned());
    let cw20_token_2 = Token::Cw20(DAODAO.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();
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
    suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &cw20_token_1.denom()),
            cw20_token_1.clone(),
            None,
            None,
            None,
        )
        .unwrap();
    suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &cw20_token_2.denom()),
            cw20_token_2.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    suite.assert_market(native_token);
    suite.assert_market(cw20_token_1);
    suite.assert_market(cw20_token_2);
}

#[test]
fn market_create_unauthorized() {
    let market_token = Token::Cw20(MARKET_TOKEN.to_owned());
    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    let err = suite
        .create_market_quick(
            "random_dude",
            &("c".to_owned() + &market_token.denom()),
            market_token,
            None,
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
}

#[test]
fn market_create_already_exists() {
    let native_token = Token::Native(JUNO.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());
    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

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
    let err = suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &native_token.denom()),
            native_token.clone(),
            None,
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::MarketAlreadyExists(native_token.denom()),
        err.downcast().unwrap()
    );

    // Fails with cw20.
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
    let err = suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &cw20_token.denom()),
            cw20_token.clone(),
            None,
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::MarketAlreadyExists(cw20_token.denom()),
        err.downcast().unwrap()
    );
}

#[test]
fn collateral_ratio_higher_then_liquidation_price() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());
    let mut suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_liquidation_price(Decimal::percent(92))
        .build();

    // Fails if collateral ratio is equal to liquidation price
    let err = suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &market_token.denom()),
            market_token.clone(),
            Decimal::percent(92),
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::MarketCfgCollateralFailure {},
        err.downcast().unwrap()
    );

    // Fails if collateral ratio is higher than liquidation price.
    let err = suite
        .create_market_quick(
            GOVERNANCE,
            &("c".to_owned() + &market_token.denom()),
            market_token,
            Decimal::percent(93),
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::MarketCfgCollateralFailure {},
        err.downcast().unwrap()
    );
}
