# Market contract

The Market contract is the entry point for all lending and borrowing for one base asset.

It is responsible for managing debt, minting and burning cTokens (collateral, allowed for lend) denominated
by one base asset. It also is able to use an oracle contract to normalize these amounts
to some "common token", which allows the credit agency to compare different markets.
A market is tied to one "credit agency" contract, which gives it a global view
of that user's history among all markets.

## Collateral

The market allows deposits of cTokens up to `market_cap` (if set). These assets may
collect interest. They can also be used as collateral, increasing the credit_limit
to borrow in other markets. This primarily impacts the

## Borrowing

The market allows borrowing of the base asset up to an account's credit limit.
The credit limit is determined by the credit agency contract.
Upon borrowing, the base asset is transferred to the account, and a corresponding
amount of debt is stored.
This impacts future queries to the Credit Agency to calculate total credit line.

## Interest

Interest is charged to debt holders and paid out to cToken holders.

The rate at which it is charged is based on "utilization rate" (what percentage
of total collateral has been borrowed). It may be a linear (compound) or piecewise
linear (Aave) curve, going up as utilization goes up, sometimes very quickly at
high utilization.

Interest is "charged" by multiplying all debt, basically, increasing the amount of debt.
Interest is "paid" by multiplying all cTokens, increasing the withdrawable collateral.
The calculations of how this is done exactly will be added later, but it ensures the
amount of collateral increases equivalent to the amount of debt (minus the "reserve" payment).

## Reserve

A percentage of the collected interest, called the `reserve_factor`, does not go
to collateral providers but rather the protocol itself. This reserve_factor is held
in the contract, but belongs to the governance contract, which can withdraw it as desired.

Note that if the contract pays out this reserve to keep liquidity in withdrawls, the
governance contract effectively becomes a cToken holder. We should actually make this explicit,
rather than just playing with interest rates, and determine if that is desired behavior,
or the best mechanism to pay the revenue to the gov contract (maybe just transfer cTokens
everytime interest is charged). TODO: future issue

## Withdrawing

Any `cTokens` can be redeemed immediately for `multiplier` base tokens (a number which
increases as interest is paid). This will fail if so much collateral has been borrowed,
such that the contract cannot honor this commitment. However, this may increase utilization
rate to 100% such that interest payments increase rapidly and encourage loan repayments.

## Governance

Privileged actions are currently defined as "sudo" messages, meant to be called by
native on-chain governance (`x/gov`). These should be converted to ExecuteMsg variants
that are only callable by a `gov_contract` defined in instantition.

These messages allow adjustments of key parameters of the market, like collateralization
rate and interest rates.
