use cosmwasm_std::StdError;
use thiserror::Error;
use utils::token::Token;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("There is no info about the prices for this trading pair: {denom1}, {denom2}")]
    NoInfo { denom1: Token, denom2: Token },
}
