use crate::utils::TestUtils;
use near_sdk_sim::{call, to_yocto};
mod utils;

#[test]
fn test_successful_payout() {
  let utils = TestUtils::init_whitelist_contracts();

  let alice = &utils.alice;
  let dao = &utils.dao;
  let contract = &utils.payouts_contract;
  let transfer_amount = to_yocto("1"); // 1 NEAR

  assert!(utils.is_whitelisted(alice.account_id.clone()));

  let result = call!(
    dao,
    contract.payout(alice.account_id.clone()),
    deposit = transfer_amount
  );
  result.assert_success();

  // Check Alice balance
  let alice_balance = utils.retrieve_account_balance(alice.account_id.as_str());
  TestUtils::assert_eq_with_gas(
    to_yocto("11"), // 10 + 1 NEAR
    alice_balance
  );

  // Check Dao balance
  let dao_balance = utils.retrieve_account_balance(dao.account_id.as_str());
  TestUtils::assert_eq_with_gas(
    to_yocto("9"), // 10 - 1 NEAR
    dao_balance
  );
}

#[test]
fn test_account_is_not_whitelisted() {
  let utils = TestUtils::init_whitelist_contracts();

  let bob = &utils.bob;
  let dao = &utils.dao;
  let contract = &utils.payouts_contract;
  let transfer_amount = to_yocto("1"); // 1 NEAR

  assert!(!utils.is_whitelisted(bob.account_id.clone()));
  let dao_balance_start = utils.retrieve_account_balance(dao.account_id.as_str());
  let bob_balance_start = utils.retrieve_account_balance(bob.account_id.as_str());

  let result = call!(
    dao,
    contract.payout(bob.account_id.clone()),
    deposit = transfer_amount
  );
  result.assert_success();

  // Check the log for callback output
  assert_eq!(result.logs().len(), 1);
  assert!(result.logs()[0].contains("ERR_RECEIVER_IS_NOT_WHITELISTED"));

  // The balance of the Dao has not changed
  let dao_balance_end = utils.retrieve_account_balance(dao.account_id.as_str());
  TestUtils::assert_eq_with_gas(dao_balance_start, dao_balance_end);

  // Bob's balance has not changed
  let bob_balance_end = utils.retrieve_account_balance(dao.account_id.as_str());
  TestUtils::assert_eq_with_gas(bob_balance_start, bob_balance_end);
}

#[test]
fn test_non_existing_account() {
  let utils = TestUtils::init_whitelist_contracts();

  let dao = &utils.dao;
  let contract = &utils.payouts_contract;
  let transfer_amount = to_yocto("1"); // 1 NEAR

  let dao_balance_start = utils.retrieve_account_balance(dao.account_id.as_str());

  let result = call!(
    dao,
    contract.payout("charlie".parse().unwrap()),
    deposit = transfer_amount
  );
  result.assert_success();

  // Check the log for callback output
  assert_eq!(result.logs().len(), 1);
  assert!(result.logs()[0].contains("ERR_RECEIVER_IS_NOT_WHITELISTED"));

  // The balance of the Dao has not changed
  let dao_balance_end = utils.retrieve_account_balance(dao.account_id.as_str());
  TestUtils::assert_eq_with_gas(dao_balance_start, dao_balance_end);
}

#[test]
fn test_account_is_whitelisted_but_deleted() {
  let utils = TestUtils::init_whitelist_contracts();

  let alice = &utils.alice;
  let dao = &utils.dao;
  let contract = &utils.payouts_contract;
  let transfer_amount = to_yocto("1"); // 1 NEAR

  assert!(utils.is_whitelisted(alice.account_id.clone()));
  utils.delete_account(alice);
  let dao_balance_start = utils.retrieve_account_balance(dao.account_id.as_str());

  let result = call!(
    dao,
    contract.payout(alice.account_id.clone()),
    deposit = transfer_amount
  );
  result.assert_success();

  // One error should occur during the promise execute
  TestUtils::assert_one_promise_error(
    result.clone(),
    "Can't complete the action because account \"alice\" doesn't exist"
  );

  // The balance of the Dao has not changed
  let dao_balance_end = utils.retrieve_account_balance(dao.account_id.as_str());
  TestUtils::assert_eq_with_gas(dao_balance_start, dao_balance_end);
}
