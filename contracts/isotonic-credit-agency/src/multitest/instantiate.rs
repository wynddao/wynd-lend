use cosmwasm_std::{Addr, Decimal};
use utils::token::Token;

use super::suite::{SuiteBuilder, COMMON};
use crate::{
    multitest::suite::{GOVERNANCE, JUNO},
    state::Config,
};

// Simple test, don't need to test all cw20, tokens combinations.
#[test]
fn market_instantiate_and_query_config() {
    let common_token = Token::Native(COMMON.to_owned());
    let reward_token = Token::Cw20(JUNO.to_owned());

    let suite = SuiteBuilder::new()
        .with_gov(GOVERNANCE)
        .with_reward_token(reward_token.clone())
        .build();

    assert_eq!(
        Config {
            gov_contract: Addr::unchecked(GOVERNANCE),
            isotonic_market_id: 3,
            isotonic_token_id: 4,
            reward_token,
            common_token,
            liquidation_price: Decimal::percent(92),
            borrow_limit_ratio: Decimal::one(),
        },
        suite.query_config().unwrap()
    );
}
