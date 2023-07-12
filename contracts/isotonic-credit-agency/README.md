# Credit Agency

The credit agency manages all the Lending Pools and authorizes loans, transfer of collateral, and, if necessary, liquidations. It depends on Price Oracles to provide accurate price feeds. It also requires a Liquidator to sell off assets. This could be swapping on an AMM or directly selling to anyone claiming the liquidation event. Those are detailed in other sections. Here, we describe how credit is calculated.

## Collateral Ratio

Different base assets have different quality and volatility. These characteristics determine how much collateral is needed relative to a loan. Thus, when adding a Lending Pool to the Credit Agency, we also need to set a collateral ratio. In a future version, this can be changed by governance, for now it is a constant.
If I have `S` value in cTokens, I can borrow `S * collateral_ratio`. 

## Common Currency

In order to compare all of these Lending Pools, we need to price all base assets to some common currency. The price oracles do this, but require a highly liquid market to provide accurate pricing (and time average it). Since we are targeting Osmosis, and most Cosmos assets have their main trading pair against OSMO, let's use OSMO as the common currency.
This means, for every Lending Pool, we need a price oracle that will provide a reliable value of the price of this asset relative to OSMO. The math works the same regardless of the common asset, this should just be adjusted to find the most common trading pairs.
Calculating Credit Line
For a given user, we can calculate their credit line by summing up the total amount of cTokens multiplied by collateral_ratio. We can find their available credit by summing the total amount of debt and subtracting it from the credit line. If the available credit is ever negative, the account may be liquidated.

## Entering Markets

While the above logic is correct, it is also quite expensive to execute if we have a few dozen different Lending Pools, while a given account only uses 2 or 3. In order to speed this up, a user can "enter" a market by declaring their intent to use it. This market is then used to calculate their Credit Line and Total Debt. Note that you can only borrow from markets you have added.

## Liquidation

Liquidation is the act of selling collateral to cover the undercollateralized debt positions. Undercollateralized means the collateral ratio is not maintained. There may still be 150% more value in the collateral than in the borrowed assets, but this margin is important to protect the lenders when the market changes quickly.
We can provide multiple liquidation strategies in the future, but the initial one is as follows:
We provide a liquidate method that can be called by anyone on any account that has negative available credit. They will pay a base asset to repay debt for the user, and in return receive the collateral from the user. In order to incentivise bots to monitor the situation and liquidate, they get the liquidated assets at an 8% discount (fixed number, configured in init). 
If the borrower has multiple cTokens as collateral, the one with the lowest collateral_ratio must be returned first, as this is the quickest way to get to a healthy ratio. You can only pay back one asset at a time, and get the equivalent amount + 8% in cTokens belonging to the user, determined by the above ratio: 
