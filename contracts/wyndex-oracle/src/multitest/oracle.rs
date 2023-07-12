use super::suite::SuiteBuilder;

use cosmwasm_std::{assert_approx_eq, coin, Decimal, Fraction, Uint128};
use wyndex::{
    asset::{Asset, AssetInfo},
    factory::PairType,
    oracle::TwapResponse,
};

use std::str::FromStr;

use crate::ContractError;
use utils::wyndex::SwapOperation;

pub const TWAP_INTERVAL: u64 = 30 * 60; // 30 minutes

#[test]
fn unauthorized_register_pool() {
    let mut suite = SuiteBuilder::new().build();

    let juno_info = AssetInfo::Native("juno".to_string());
    let atom_info = AssetInfo::Native("ibc".to_string());

    let juno_atom = suite.wyndex.create_pair(
        &mut suite.app,
        &[juno_info.clone(), atom_info.clone()],
        PairType::Xyk {},
        None,
    );

    let err = suite
        .register_pool("sender", juno_atom.as_str(), &juno_info, &atom_info)
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
}

#[test]
fn query_unregistered_pool() {
    let mut suite = SuiteBuilder::new().with_controller("controller").build();

    let juno_info = AssetInfo::Native("juno".to_string());
    let atom_info = AssetInfo::Native("ibc".to_string());

    let _juno_atom = suite.wyndex.create_pair(
        &mut suite.app,
        &[juno_info.clone(), atom_info.clone()],
        PairType::Xyk {},
        None,
    );

    // Querying for pool address works both ways
    let err = suite
        .query_pool_address(&juno_info, &atom_info)
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("There is no info about the contract address of pair"));
}

#[test]
fn register_pool() {
    let mut suite = SuiteBuilder::new().with_controller("controller").build();

    let juno_info = AssetInfo::Native("juno".to_string());
    let atom_info = AssetInfo::Native("ibc".to_string());

    let juno_atom = suite.wyndex.create_pair(
        &mut suite.app,
        &[juno_info.clone(), atom_info.clone()],
        PairType::Xyk {},
        None,
    );

    // "controller" here represents an address that is allowed to register pools
    suite
        .register_pool("controller", juno_atom.as_str(), &juno_info, &atom_info)
        .unwrap();

    // Querying for pool address works both ways
    let result = suite.query_pool_address(&juno_info, &atom_info).unwrap();
    assert_eq!(juno_atom, result);
    let result = suite.query_pool_address(&atom_info, &juno_info).unwrap();
    assert_eq!(juno_atom, result);
}

#[test]
fn query_price_empty_pool() {
    let mut suite = SuiteBuilder::new().with_controller("controller").build();

    let juno_info = AssetInfo::Native("juno".to_string());
    let atom_info = AssetInfo::Native("ibc".to_string());

    let juno_atom = suite.wyndex.create_pair(
        &mut suite.app,
        &[juno_info.clone(), atom_info.clone()],
        PairType::Xyk {},
        None,
    );

    suite
        .register_pool("controller", juno_atom.as_str(), &juno_info, &atom_info)
        .unwrap();

    // Querying for pool address works both ways
    let err = suite.query_twap(&juno_info, &atom_info).unwrap_err();
    assert!(err
        .to_string()
        .contains("Querier contract error: wyndex::oracle::LastUpdates not found"));
}

#[test]
fn query_twap_price() {
    let atom = "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9";
    let mut suite = SuiteBuilder::new()
        .with_funds(
            "owner",
            &[coin(600_000_000, "juno"), coin(200_000_000, atom)],
        )
        .with_controller("controller")
        .build();

    let pool_amount = 100_000_000u128;
    let pool_ratio = 3u128;
    let juno_info = AssetInfo::Native("juno".to_string());
    let atom_info = AssetInfo::Native(atom.to_string());

    let juno_atom = suite.wyndex.create_pair(
        &mut suite.app,
        &[juno_info.clone(), atom_info.clone()],
        PairType::Xyk {},
        None,
    );

    // provide liquidity and wait for prices to be collected
    suite
        .wyndex
        .provide_liquidity(
            &mut suite.app,
            "owner",
            &juno_atom,
            &[
                Asset {
                    info: juno_info.clone(),
                    amount: Uint128::new(pool_amount * pool_ratio),
                },
                Asset {
                    info: atom_info.clone(),
                    amount: Uint128::new(pool_amount),
                },
            ],
            &[
                coin(pool_amount * pool_ratio, "juno"),
                coin(pool_amount, atom),
            ],
        )
        .unwrap();
    suite.advance_seconds(TWAP_INTERVAL);
    suite
        .wyndex
        .provide_liquidity(
            &mut suite.app,
            "owner",
            &juno_atom,
            &[
                Asset {
                    info: juno_info.clone(),
                    amount: Uint128::new(pool_amount * pool_ratio),
                },
                Asset {
                    info: atom_info.clone(),
                    amount: Uint128::new(pool_amount),
                },
            ],
            &[
                coin(pool_amount * pool_ratio, "juno"),
                coin(pool_amount, atom),
            ],
        )
        .unwrap();
    suite.advance_seconds(TWAP_INTERVAL);

    // "controller" here represents an address that is allowed to register pools
    suite
        .register_pool("controller", juno_atom.as_str(), &juno_info, &atom_info)
        .unwrap();

    let response = suite.query_twap(&juno_info, &atom_info).unwrap();
    assert_eq!(
        response,
        TwapResponse {
            a: juno_info,
            b: atom_info,
            a_per_b: Decimal::from_ratio(pool_ratio, 1u128),
            // b_per_a: Decimal::from_ratio(1u128, pool_ratio), // to big precision making
            // assertion fail
            b_per_a: Decimal::from_str("0.333333333").unwrap(),
        }
    );
}

#[test]
fn simulate_direct_swap() {
    let mut suite = SuiteBuilder::new()
        .with_funds(
            "owner",
            &[coin(600_000_000, "juno"), coin(200_000_000, "atom")],
        )
        .with_controller("controller")
        .build();

    let juno_info = AssetInfo::Native("juno".to_string());
    let atom_info = AssetInfo::Native("atom".to_string());

    let juno_atom = suite.wyndex.create_pair(
        &mut suite.app,
        &[juno_info.clone(), atom_info.clone()],
        PairType::Xyk {},
        None,
    );

    // provide liquidity and wait for prices to be collected
    suite
        .wyndex
        .provide_liquidity(
            &mut suite.app,
            "owner",
            &juno_atom,
            &[
                Asset {
                    info: juno_info.clone(),
                    amount: Uint128::new(250_000u128),
                },
                Asset {
                    info: atom_info.clone(),
                    amount: Uint128::new(100_000u128),
                },
            ],
            &[coin(250_000u128, "juno"), coin(100_000u128, "atom")],
        )
        .unwrap();

    // One atom costs 2.5 juno, so swapping 1000 atom should give us ~2500 juno
    let response = suite
        .simulate_swap_operations(
            1_000u128,
            vec![SwapOperation::WyndexSwap {
                offer_asset_info: atom_info.clone(),
                ask_asset_info: juno_info.clone(),
            }],
        )
        .unwrap();
    let spread = Decimal::percent(1);
    assert_eq!(response.spread, spread);
    assert_approx_eq!(
        response.amount,
        Uint128::new(2_500u128),
        &spread.to_string()
    );

    // Now in reverse
    // asking how many atom do I need to get 2500 juno
    let response = suite
        .simulate_reverse_swap_operations(
            2_500u128,
            vec![SwapOperation::WyndexSwap {
                ask_asset_info: juno_info,
                offer_asset_info: atom_info,
            }],
        )
        .unwrap();
    let spread = Decimal::percent(1);
    assert_approx_eq!(response.spread.numerator(), spread.numerator(), "0.01");
    assert_approx_eq!(
        response.amount,
        Uint128::new(1_000u128),
        &spread.to_string()
    );
}
