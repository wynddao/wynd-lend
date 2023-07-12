// Copied from github.com/wynddao/wynddex
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use wyndex::asset::{AssetInfo, AssetValidated};

#[cw_serde]
pub enum MultiHopQueryMsg {
    SimulateSwapOperations {
        /// The amount of tokens to swap
        offer_amount: Uint128,
        /// The swap operations to perform, each swap involving a specific pool
        operations: Vec<SwapOperation>,
        /// Whether to simulate referral
        referral: bool,
        /// The commission for the referral. Only used if `referral` is set to `true`.
        /// This is capped by and defaulting to the configured max commission.
        /// The commission is only applied to the first of these swap operations,
        /// so the referrer will get a portion of the asset the swap starts with.
        referral_commission: Option<Decimal>,
    },
    SimulateReverseSwapOperations {
        /// The amount of tokens to receive
        ask_amount: Uint128,
        /// The swap operations to perform, each swap involving a specific pool.
        /// This is *not* in reverse order. It starts with the offer asset and ends with the ask asset.
        operations: Vec<SwapOperation>,
        /// Whether to simulate referral
        referral: bool,
        /// The commission for the referral. Only used if `referral` is set to `true`.
        /// This is capped by and defaulting to the configured max commission.
        /// The commission is only applied to the first of these swap operations,
        /// so the referrer will get a portion of the asset the swap starts with.
        referral_commission: Option<Decimal>,
    },
}

/// This enum describes a swap operation.
#[cw_serde]
pub enum SwapOperation {
    /// Wyndex swap
    WyndexSwap {
        /// Information about the asset being swapped
        offer_asset_info: AssetInfo,
        /// Information about the asset we swap to
        ask_asset_info: AssetInfo,
    },
}

/// This structure describes a custom struct to return a query response containing the end amount of a swap simulation
#[cw_serde]
pub struct SimulateSwapOperationsResponse {
    /// The amount of tokens received / offered in a swap simulation
    pub amount: Uint128,

    /// The spread percentage for the whole all swap operations as a whole.
    /// This is the percentage by which the returned `amount` is worse than the ideal one.
    pub spread: Decimal,

    /// The absolute amounts of spread for each swap operation.
    /// This contains one entry per swap operation in the same order as the `operations` parameter,
    /// and each entry is denominated in the asset that is swapped to (`ask_asset_info`).
    pub spread_amounts: Vec<AssetValidated>,

    /// The absolute amounts of commission for each swap operation.
    /// This contains one entry per swap operation in the same order as the `operations` parameter,
    /// and each entry is denominated in the asset that is swapped to (`ask_asset_info`).
    pub commission_amounts: Vec<AssetValidated>,

    /// The absolute amount of referral commission. This is always denominated in `offer_asset_info`.
    pub referral_amount: AssetValidated,
}

#[cw_serde]
pub enum ExecuteMsg {
    ExecuteSwapOperations {
        /// All swap operations to perform
        operations: Vec<SwapOperation>,
        /// Guarantee that the ask amount is above or equal to a minimum amount
        minimum_receive: Option<Uint128>,
        /// Recipient of the ask tokens
        receiver: Option<String>,
        max_spread: Option<Decimal>,
        /// The address that should receive the referral commission
        referral_address: Option<String>,
        /// The commission for the referral.
        /// This is capped by the configured max commission
        referral_commission: Option<Decimal>,
    },
}
