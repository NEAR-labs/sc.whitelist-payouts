use near_sdk::{AccountId, Balance};
use near_sdk::serde_json::json;
use near_sdk_sim::transaction::ExecutionStatus;
use near_sdk_sim::{
   deploy,
   init_simulator,
   lazy_static_include,
   to_yocto,
   ContractAccount,
   DEFAULT_GAS,
   ExecutionResult,
   UserAccount
};
use whitelist_payouts::WhitelistPayoutsContract;

const SPUTNIK_FACTORY_CONTRACT_ID: &str = "sputnik";
const SMART_WHITELIST_CONTRACT_ID: &str = "smart-whitelist";
const WHITELIST_PAYOUTS_CONTRACT_ID: &str = "whitelist-payouts";
const DEFAULT_BALANCE: &str = "10";
const CONTRACT_BALANCE: &str = "20";
const NO_DEPOSIT: Balance = 0;

   lazy_static_include::lazy_static_include_bytes! {
   SMART_WHITELIST_BYTES => "tests/sim/external_wasm/smart_whitelist.wasm",
   WHITELIST_PAYOUTS_BYTES => "wasm/whitelist_payouts.wasm",
}

pub struct TestUtils {
   pub root: UserAccount,
   pub payouts_contract: ContractAccount<WhitelistPayoutsContract>,
   pub whitelist: UserAccount,
   pub dao: UserAccount,
   pub alice: UserAccount,
   pub bob: UserAccount,
}

impl TestUtils {
   pub fn init_whitelist_contracts() -> Self {
      let root = init_simulator(None);

      // Smart-whitelist contract
      let whitelist = root.deploy(
         &SMART_WHITELIST_BYTES,
         SMART_WHITELIST_CONTRACT_ID.parse().unwrap(),
         to_yocto(CONTRACT_BALANCE),
      );

      // Whitelist payouts contract
      let payouts_contract = deploy!(
      contract: WhitelistPayoutsContract,
      contract_id: WHITELIST_PAYOUTS_CONTRACT_ID,
      bytes: &WHITELIST_PAYOUTS_BYTES,
      signer_account: root,
      deposit: to_yocto(CONTRACT_BALANCE),
      init_method: new(
            SPUTNIK_FACTORY_CONTRACT_ID.parse().unwrap(),
            whitelist.account_id.clone()
         )
      );

      let factory = Self::create_user(
         &root,
         SPUTNIK_FACTORY_CONTRACT_ID.to_string(),
         CONTRACT_BALANCE
      );
      let dao = Self::create_user(
         &factory,
         format!("dao.{}", SPUTNIK_FACTORY_CONTRACT_ID),
         DEFAULT_BALANCE
      );
      let alice = Self::create_user(
         &root,
         "alice".to_string(),
         DEFAULT_BALANCE
      );
      Self::init_whitelist(&root, &whitelist, &alice);
      let bob = Self::create_user(
         &root,
         "bob".to_string(),
         DEFAULT_BALANCE
      );

      TestUtils {
         root,
         payouts_contract,
         whitelist,
         dao,
         alice,
         bob,
      }
   }

   pub fn is_whitelisted(&self, account_id: AccountId) -> bool {
      self.root.view(
         self.whitelist.account_id.clone(),
         "is_whitelisted",
         &json!({
            "account_id": account_id
         })
           .to_string()
           .into_bytes()
      ).unwrap_json()
   }

   pub fn retrieve_account_balance(&self, account_id: &str) -> Balance {
      self.root
        .borrow_runtime()
        .view_account(account_id)
        .unwrap()
        .amount
   }

   pub fn assert_almost_eq_with_max_delta(left: u128, right: u128, max_delta: u128) {
      assert!(
         std::cmp::max(left, right) - std::cmp::min(left, right) <= max_delta,
         "{}",
         format!(
            "Left {} is not even close to Right {} within delta {}",
            left, right, max_delta
         )
      );
   }

   pub fn assert_eq_with_gas(left: u128, right: u128) {
      Self::assert_almost_eq_with_max_delta(left, right, to_yocto("0.03")); // 300 Tgas
   }

   pub fn delete_account(&self, account: &UserAccount) {
      account.create_transaction(account.account_id.clone())
        .delete_account(self.root.account_id.clone())
        .submit();
   }

   pub fn assert_one_promise_error(promise_result: ExecutionResult, expected_error_message: &str) {
      assert_eq!(promise_result.promise_errors().len(), 1);

      if let ExecutionStatus::Failure(execution_error) =
      &promise_result.promise_errors().remove(0).unwrap().outcome().status
      {
         assert!(execution_error.to_string().contains(expected_error_message));
      } else {
         unreachable!();
      }
   }

   fn init_whitelist(root: &UserAccount, whitelist: &UserAccount, user: &UserAccount) {
      let admin_account = Self::create_user(
         root,
         "whitelist-admin".to_string(),
         DEFAULT_BALANCE
      );
      whitelist.call(
         whitelist.account_id.clone(),
         "new",
         &json!({
            "admin_pk": admin_account.signer.public_key
      })
           .to_string()
           .into_bytes(),
         DEFAULT_GAS,
         NO_DEPOSIT,
      ).assert_success();

      let service_account = Self::create_user(
         root,
         "whitelist-service".to_string(),
         DEFAULT_BALANCE
      );
      admin_account.call(
         whitelist.account_id.clone(),
         "add_service_account",
         &json!({
            "service_account_id": service_account.account_id
         })
           .to_string()
           .into_bytes(),
         DEFAULT_GAS,
         NO_DEPOSIT,
      ).assert_success();

      user.call(
         whitelist.account_id.clone(),
         "register_applicant",
         "{}".as_bytes(),
         DEFAULT_GAS,
         NO_DEPOSIT,
      ).assert_success();

      service_account.call(
         whitelist.account_id.clone(),
         "add_account",
         &json!({
            "account_id": user.account_id
         })
           .to_string()
           .into_bytes(),
         DEFAULT_GAS,
         NO_DEPOSIT,
      ).assert_success();
   }

   fn create_user(owner: &UserAccount, name: String, initial_balance: &str) -> UserAccount {
      owner.create_user(AccountId::new_unchecked(name), to_yocto(initial_balance))
   }
}
