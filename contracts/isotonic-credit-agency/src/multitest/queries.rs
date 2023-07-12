use utils::token::Token;

use crate::multitest::suite::{ATOM, GOVERNANCE, JUNO, MARKET_TOKEN, WYND};

use super::suite::SuiteBuilder;

#[test]
fn query_market() {
    let native_token = Token::Native(JUNO.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

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
    let res = suite.query_market(native_token.clone()).unwrap();
    assert_eq!(res.market_token, native_token);

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
    let res = suite.query_market(cw20_token.clone()).unwrap();
    assert_eq!(res.market_token, cw20_token);
}

#[test]
fn query_market_does_not_exist() {
    let market_token = Token::Native(MARKET_TOKEN.to_owned());

    let suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    let err = suite.query_market(market_token.clone()).unwrap_err();
    let expected_error_suffix = format!("No market set up for base asset {}", market_token.denom());
    assert!(err.to_string().ends_with(&expected_error_suffix));
}

#[test]
fn list_markets() {
    let native_token_1 = Token::Native(JUNO.to_owned());
    let native_token_2 = Token::Native(ATOM.to_owned());
    let cw20_token = Token::Cw20(WYND.to_owned());

    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

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
            &cw20_token.denom(),
            cw20_token.clone(),
            None,
            None,
            None,
        )
        .unwrap();

    let mut list: Vec<_> = suite
        .list_markets()
        .unwrap()
        .markets
        .into_iter()
        .map(|r| r.market_token)
        .collect();
    list.sort();

    assert_eq!(list, [native_token_2, native_token_1, cw20_token]);
}

#[test]
fn list_markets_empty_list() {
    let suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    let res = suite.list_markets().unwrap();
    assert_eq!(res.markets, []);
}

fn generate_denoms(prefix: &str, start: u32, end: u32) -> Vec<Token> {
    (start..end)
        .map(|i| Token::Native(format!("{}{:02}", prefix, i)))
        .collect()
}

#[test]
fn list_markets_default_pagination() {
    let mut suite = SuiteBuilder::new().with_gov("gov").build();

    // create markets for native tokens "TOKEN00", "TOKEN01", ..., "TOKEN14"
    // the default pagination limit is 10 entries per page
    for i in 0..15 {
        suite
            .create_market_quick(
                "gov",
                &format!("token{:02}", i),
                Token::Native(format!("TOKEN{:02}", i)),
                None,
                None,
                None,
            )
            .unwrap();
    }

    let mut list1: Vec<_> = suite
        .list_markets()
        .unwrap()
        .markets
        .into_iter()
        .map(|r| r.market_token)
        .collect();
    list1.sort();
    assert_eq!(list1, generate_denoms("TOKEN", 0, 10));

    let mut list2: Vec<_> = suite
        .list_markets_with_pagination(list1.last().unwrap().clone(), None)
        .unwrap()
        .markets
        .into_iter()
        .map(|r| r.market_token)
        .collect();
    list2.sort();
    assert_eq!(list2, generate_denoms("TOKEN", 10, 15));
}

#[test]
fn list_markets_custom_pagination() {
    let mut suite = SuiteBuilder::new().with_gov(GOVERNANCE).build();

    // create markets for native tokens "TOKEN00", "TOKEN01", ..., "TOKEN05"
    // we set the pagination limit to 3 entries per page
    for i in 0..5 {
        suite
            .create_market_quick(
                GOVERNANCE,
                &format!("token{:02}", i),
                Token::Native(format!("TOKEN{:02}", i)),
                None,
                None,
                None,
            )
            .unwrap();
    }

    let mut list1: Vec<_> = suite
        .list_markets_with_pagination(None, 3)
        .unwrap()
        .markets
        .into_iter()
        .map(|r| r.market_token)
        .collect();
    list1.sort();
    assert_eq!(list1, generate_denoms("TOKEN", 0, 3));

    let mut list2: Vec<_> = suite
        .list_markets_with_pagination(list1.last().unwrap().clone(), 3)
        .unwrap()
        .markets
        .into_iter()
        .map(|r| r.market_token)
        .collect();
    list2.sort();
    assert_eq!(list2, generate_denoms("TOKEN", 3, 5));
}
