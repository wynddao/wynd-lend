use cosmwasm_std::{
    to_binary, BankMsg, Coin as StdCoin, coin, CosmosMsg, CustomQuery, Deps, StdError, StdResult, Uint128, WasmMsg, Addr, Decimal,
};
use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use wyndex::asset::AssetInfo;

use crate::{wyndex::{ExecuteMsg::ExecuteSwapOperations, SwapOperation}, coin::{self, Coin}};

use std::fmt;

/// Universal token type which is either a native token, or cw20 token
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, PartialOrd, Ord, Hash,
)]
pub enum Token {
    /// Native token of given name
    Native(String),
    /// Cw20 token with its cw20 contract address
    Cw20(String),
}

impl Token {
    pub fn new_native(denom: &str) -> Self {
        Self::Native(denom.to_owned())
    }

    pub fn new_cw20(denom: &str) -> Self {
        Self::Cw20(denom.to_owned())
    }

    /// Return native token name or `None`
    pub fn native(self) -> Option<String> {
        match self {
            Token::Native(token) => Some(token),
            _ => None,
        }
    }

    /// Returns cw20 token address or `None`
    pub fn cw20(self) -> Option<String> {
        match self {
            Token::Cw20(addr) => Some(addr),
            _ => None,
        }
    }

    /// Return native token name or `None`
    pub fn as_native(&self) -> Option<&str> {
        match self {
            Token::Native(token) => Some(token),
            _ => None,
        }
    }

    /// Returns cw20 token address or `None`
    pub fn as_cw20(&self) -> Option<&str> {
        match self {
            Token::Cw20(addr) => Some(addr),
            _ => None,
        }
    }

    /// Checks if token is native
    pub fn is_native(&self) -> bool {
        matches!(self, Token::Native(_))
    }

    /// Checks i token is cw20
    pub fn is_cw20(&self) -> bool {
        matches!(self, Token::Cw20(_))
    }

    /// Helper function to return the Address of the CW20 token or the denom of the native one.
    pub fn denom(&self) -> String {
        use Token::*;
        match self {
            Native(denom) => denom.clone(),
            Cw20(denom) => denom.clone(),
        }
    }

    /// Queries the balance of the given address
    pub fn query_balance<T: CustomQuery>(
        &self,
        deps: Deps<'_, T>,
        address: impl Into<String>,
    ) -> StdResult<u128> {
        Ok(match self {
            Self::Native(denom) => deps.querier.query_balance(address, denom)?.amount.into(),
            Self::Cw20(cw20_token) => deps
                .querier
                .query_wasm_smart::<cw20::BalanceResponse>(
                    cw20_token,
                    &cw20::Cw20QueryMsg::Balance {
                        address: address.into(),
                    },
                )?
                .balance
                .into(),
        })
    }

    pub fn amount(&self, amount: impl Into<Uint128>) -> Coin {
        Coin {
            amount: amount.into(),
            denom: self.clone(),
        }
    }

    /// Helper function to create a custom `utils::coin::Coin` from a `Token`.
    pub fn into_coin(self, amount: impl Into<Uint128>) -> Coin {
        Coin {
            amount: amount.into(),
            denom: self,
        }
    }

    /// Creates a send message for this token to send the given amount from this contract to the given address
    pub fn send_msg<T>(
        &self,
        to_address: impl Into<String>,
        amount: impl Into<Uint128>,
    ) -> StdResult<CosmosMsg<T>> {
        Ok(match self {
            Self::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
                to_address: to_address.into(),
                amount: vec![cosmwasm_std::Coin {
                    denom: denom.clone(),
                    amount: amount.into(),
                }],
            }),
            Self::Cw20(address) => CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: address.to_owned(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: to_address.into(),
                    amount: amount.into(),
                })?,
                funds: vec![],
            }),
        })
    }

    pub fn swap_msg<T>(
        &self,
        multi_hop: Addr,
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        sell_limit: Uint128,
    ) -> StdResult<CosmosMsg<T>> {
        Ok(match self {
            Self::Native(denom) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: multi_hop.to_string(),
                msg: to_binary(&ExecuteSwapOperations {
                    operations,
                    // Minimum accepted is precisely what we want to buy.
                    minimum_receive,
                    // This implies sender of the message.
                    receiver: None,
                    max_spread: None,
                    referral_address: None,
                    referral_commission: None,
                })?,
                funds: vec![coin(sell_limit.u128(), denom)],
            }),
            Self::Cw20(address) => unimplemented!()
        })
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Native(s) => write!(f, "{}", s),
            Token::Cw20(s) => write!(f, "{}", s),
        }
    }
}

impl From<AssetInfo> for Token {
    fn from(asset_info: AssetInfo) -> Self {
        match asset_info {
            AssetInfo::Native(denom) => Token::Native(denom),
            AssetInfo::Token(address) => Token::Cw20(address),
        }
    }
}

impl From<Token> for AssetInfo {
    fn from(token: Token) -> Self {
        match token {
            Token::Native(denom) => AssetInfo::Native(denom),
            Token::Cw20(address) => AssetInfo::Token(address),
        }
    }
}

impl KeyDeserialize for &Token {
    type Output = Token;

    const KEY_ELEMS: u16 = 2;

    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        let (asset_type, denom) = <(u8, &str)>::from_vec(value)?;

        match asset_type {
            0 => Ok(Token::Native(denom)),
            1 => Ok(Token::Cw20(denom)),
            _ => Err(StdError::generic_err("Invalid Token key, invalid type")),
        }
    }
}

impl<'a> Prefixer<'a> for &Token {
    fn prefix(&self) -> Vec<Key> {
        self.key()
    }
}

// Allow using `AssetInfoValidated` as a key in a `Map`
impl<'a> PrimaryKey<'a> for &Token {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<Key> {
        match self {
            Token::Native(denom) => {
                vec![Key::Val8([0]), Key::Ref(denom.as_bytes())]
            }
            Token::Cw20(addr) => vec![Key::Val8([1]), Key::Ref(addr.as_bytes())],
        }
    }
}
