use std::ops::Mul;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{
  env,
  ext_contract,
  is_promise_success,
  near_bindgen,
  AccountId,
  Balance,
  Gas,
  PanicOnDefault,
  Promise,
  PromiseError,
};

const NO_DEPOSIT: Balance = 0;
const CALLBACK: Gas = Gas(25_000_000_000_000);
const CHECK_CALL_GAS: Gas = Gas(5_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct WhitelistPayouts {
  sputnik_factory: AccountId,
  whitelist_contract: AccountId,
}

#[ext_contract(ext_self)]
pub trait ExtWhitelistPayouts {
  fn on_whitelisted(
    &mut self,
    #[callback_result] is_whitelisted: Result<bool, PromiseError>,
    account_id: AccountId,
    amount: U128,
    predecessor_account_id: AccountId,
  ) -> bool;

  fn on_transferred(
    &mut self,
    account_id: AccountId,
    amount: U128,
    predecessor_account_id: AccountId,
  ) -> bool;
}

#[near_bindgen]
impl WhitelistPayouts {
  #[init]
  pub fn new(sputnik_factory: AccountId, whitelist_contract: AccountId) -> Self {
    Self {
      sputnik_factory,
      whitelist_contract,
    }
  }

  #[payable]
  pub fn payout(&mut self, account_id: AccountId) -> Promise {
    assert!(
      env::predecessor_account_id()
        .as_str()
        .ends_with(format!(".{}", self.sputnik_factory).as_str()),
      "ERR_CALLED_ONLY_BY_FACTORY_SUB-ACCOUNT"
    );
    assert!(
      env::attached_deposit() > 0,
      "ERR_DEPOSIT_AMOUNT_CANNOT_BE_ZERO"
    );

    Promise::new(self.whitelist_contract.clone())
      .function_call(
        "is_whitelisted".to_string(),
        json!({ "account_id": account_id })
          .to_string()
          .into_bytes(),
        NO_DEPOSIT,
        CHECK_CALL_GAS, // 5 TGas
      )
      .then(ext_self::on_whitelisted(
        account_id,
        U128::from(env::attached_deposit()),
        env::predecessor_account_id(),
        env::current_account_id(),
        NO_DEPOSIT,
        CALLBACK.mul(2), // 50 TGas
      ))
  }

  #[private]
  pub fn on_whitelisted(
    &mut self,
    #[callback_result] is_whitelisted: Result<bool, PromiseError>,
    account_id: AccountId,
    amount: U128,
    predecessor_account_id: AccountId,
  ) -> bool {
    let has_whitelisted = is_promise_success() && match is_whitelisted {
      Ok(v) => v,
      _ => false
    };

    if has_whitelisted {
      Promise::new(account_id.clone())
        .transfer(amount.0)
        .then(ext_self::on_transferred(
          account_id,
          amount,
          predecessor_account_id,
          env::current_account_id(),
          NO_DEPOSIT,
          CALLBACK, // 25 TGas
        ));
      true

    } else {
      env::log_str("ERR_RECEIVER_IS_NOT_WHITELISTED");
      Promise::new(predecessor_account_id).transfer(amount.0);
      false
    }
  }

  #[private]
  pub fn on_transferred(
    &mut self,
    account_id: AccountId,
    amount: U128,
    predecessor_account_id: AccountId,
  ) -> bool {
    if is_promise_success() {
      env::log_str(&json!({
            "amount": amount,
            "payer": predecessor_account_id,
            "receiver": account_id
        })
        .to_string()
        .as_str(),
      );
      true
    } else {
      env::log_str("ERR_TRANSFERRING_TO_RECEIVER_ACCOUNT");
      Promise::new(predecessor_account_id).transfer(amount.0);
      false
    }
  }
}

#[cfg(test)]
mod tests {
  use near_sdk::test_utils::test_env::alice;
  use near_sdk::test_utils::VMContextBuilder;
  use near_sdk::{testing_env, VMContext};
  use near_sdk_sim::to_yocto;
  use super::*;

  fn get_context(predecessor_account_id: AccountId, amount: Balance) -> VMContext {
    VMContextBuilder::new()
      .predecessor_account_id(predecessor_account_id)
      .attached_deposit(amount)
      .build()
  }

  fn sputnik_factory_account() -> AccountId {
    AccountId::new_unchecked("sputnik.near".to_string())
  }

  fn whitelist_account() -> AccountId {
    AccountId::new_unchecked("whitelist.near".to_string())
  }

  fn not_dao_account() -> AccountId {
    AccountId::new_unchecked("dao.near".to_string())
  }

  fn dao_account() -> AccountId {
    AccountId::new_unchecked("dao.sputnik.near".to_string())
  }

  #[test]
  #[should_panic(expected = "ERR_CALLED_ONLY_BY_FACTORY_SUB-ACCOUNT")]
  fn test_not_caused_by_dao_account() {
    let context = get_context(not_dao_account(), to_yocto("1"));
    testing_env!(context);
    let mut contract = WhitelistPayouts::new(
      sputnik_factory_account(),
      whitelist_account()
    );
    contract.payout(alice());
  }

  #[test]
  #[should_panic(expected = "ERR_DEPOSIT_AMOUNT_CANNOT_BE_ZERO")]
  fn test_zero_deposit() {
    let context = get_context(dao_account(), 0);
    testing_env!(context);
    let mut contract = WhitelistPayouts::new(
      sputnik_factory_account(),
      whitelist_account()
    );
    contract.payout(alice());
  }
}
