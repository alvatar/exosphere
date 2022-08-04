// TODO: take into account different decimals in the tokens
// TODO: checks
// TODO: withdraw

use near_contract_standards::fungible_token::{metadata::FungibleTokenMetadata,
					      receiver::FungibleTokenReceiver,
					      FungibleToken};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen, AccountId, PromiseOrValue};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Exosphere {
    // Accounts deposits on the contract
    pub tokens: LookupMap<AccountId, (FungibleToken, FungibleTokenMetadata)>,
}

// Use FT.ft_transfer_call to send tokens from FT to pool
#[near_bindgen]
impl FungibleTokenReceiver for Exosphere {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        #[allow(unused_variables)] msg: String,
    ) -> PromiseOrValue<U128> {
        let token_name = &env::predecessor_account_id();
        let mut token = self.tokens.get(token_name).expect("Token not supported");
        token.0.internal_deposit(&sender_id, amount.0);
        self.tokens.insert(token_name, &token);
        PromiseOrValue::Value(U128::from(0_u128))
    }
}

#[near_bindgen]
impl Exosphere {

    #[init]
    pub fn new(
        token_1_contract: AccountId,
        token_2_contract: AccountId,
        token_1_metadata: FungibleTokenMetadata,
        token_2_metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Pool already initialized");

        let owner_id = env::current_account_id();

	let mut token_1 = FungibleToken::new(b"1#".to_vec());
	token_1.internal_register_account(&owner_id);
	let mut token_2 = FungibleToken::new(b"2#".to_vec());
	token_2.internal_register_account(&owner_id);

        let mut tokens = LookupMap::new(b"ExosphereAMM".to_vec());
        tokens.insert(&token_1_contract, &(token_1, token_1_metadata));
        tokens.insert(&token_2_contract, &(token_2, token_2_metadata));

        Self { tokens }
    }

    pub fn provide(
        &mut self,
        token_1_name: AccountId,
        token_1_amount: U128,
        token_2_name: AccountId,
        token_2_amount: U128,
    ) {
        let mut token_1 = self.tokens.get(&token_1_name).expect("Unknown token");
        let mut token_2 = self.tokens.get(&token_2_name).expect("Unknown token");
        let pool_owner = env::current_account_id();
        let sender = env::predecessor_account_id();

        let token_1_pool_bal = token_1.0.internal_unwrap_balance_of(&pool_owner);
        let token_2_pool_bal = token_2.0.internal_unwrap_balance_of(&pool_owner);

        if token_1_pool_bal * token_2_amount.0 == token_2_pool_bal * token_1_amount.0 {
            token_1.0.internal_transfer(&sender, &pool_owner, token_1_amount.0, None);
            token_2.0.internal_transfer(&sender, &pool_owner, token_2_amount.0, None);

            self.tokens.insert(&token_1_name, &token_1);
            self.tokens.insert(&token_2_name, &token_2);
        } else {
            panic!("token amounts should keep the pool balanced")
        }
    }

    pub fn swap(&mut self, buy_token_id: AccountId, sell_token_id: AccountId, sell_token_amount: U128) -> U128 {
        let mut buy_token = self.tokens.get(&buy_token_id).expect("Wrong token");
        let mut sell_token = self.tokens.get(&sell_token_id).expect("Wrong token");
        let pool_owner = env::current_account_id();
        let sender = env::predecessor_account_id();

        let token_sell_pool_bal = sell_token.0.internal_unwrap_balance_of(&pool_owner);
        let token_buy_pool_bal = buy_token.0.internal_unwrap_balance_of(&pool_owner);

	// TODO: check balances
        sell_token.0.internal_transfer(&sender, &pool_owner, sell_token_amount.0, None);

        let buy_token_amount = get_buy_token_amount(token_sell_pool_bal, token_buy_pool_bal, sell_token_amount.0);
        buy_token.0.internal_transfer(&pool_owner, &sender, buy_token_amount, None);

        // Update tokens data in lookup map
        self.tokens.insert(&buy_token_id, &buy_token);
        self.tokens.insert(&sell_token_id, &sell_token);

        U128::from(buy_token_amount)
    }
}

pub fn get_buy_token_amount(x: u128, y: u128, dx: u128) -> u128 {
    y - (x * y / (x + dx))
}
