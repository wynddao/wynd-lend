# Token contract

This contract is used for keeping track of CTokens.
A market contract instantiates this for tracking the collateral.

## Overview

The `***-token` contract has a few main functionalities:

* Modified cw20-token
  * Some amount of tokens may be locked
  * Transfer and burn limits are controlled by external logic (market contract)
  * Maintains a global "multiplier" of how many "base" tokens each token represents 
* ERC2222-inspired distribution contract
  * Efficiently distributes rewards among all token holders
  * Each holder can withdraw his/her share manually

## CW20 Token

The initialization can config queries are very similar to cw20-base. The currently miss
some fields like `MarketingInfo`, which could be added. Also, we could fill in the
Minter field in query response to show the controller can mint unlimited tokens.

It supports `transfer`, `transfer_from`, `send` like cw20-base.  
`mint` and `burn` are reserved for the controller.  
Allowances are not implemented as deemed not very important, but could be if desired.  
Other missing features may be an oversight, please raise issues.  

## Distribution

This is lazy distribution of tokens, just like in `wynd-stake`. This implementation
only supports Native tokens, while `wynd-stake` only supports cw20. It would be good
to refactor this into it's own module that can be embedded in different contracts,
and support both token types.