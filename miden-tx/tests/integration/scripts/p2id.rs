use miden_lib::notes::create_p2id_note;
use miden_objects::{
    accounts::{Account, AccountId},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::RpoRandomCoin,
    transaction::TransactionArgs,
    utils::collections::Vec,
    Felt,
};
use miden_tx::TransactionExecutor;
use mock::constants::{
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
    ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER, DEFAULT_AUTH_SCRIPT,
};

use crate::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map,
    prove_and_verify_transaction, MockDataStore,
};

// P2ID TESTS
// ===============================================================================================
// We test the Pay to ID script. So we create a note that can only be consumed by the target
// account.
#[test]
fn prove_p2id_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_p2id_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();

    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_sk_pk_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, Some(tx_args_target))
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let (malicious_pub_key, malicious_keypair_felt) = get_new_key_pair_with_advice_map();
    let malicious_account =
        get_account_with_default_account_code(malicious_account_id, malicious_pub_key, None);

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]));
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());
    executor_2.load_account(malicious_account_id).unwrap();
    let tx_script_malicious = executor
        .compile_tx_script(
            tx_script_code,
            vec![(malicious_pub_key, malicious_keypair_felt)],
            vec![],
        )
        .unwrap();

    let tx_args_malicious = TransactionArgs::new(Some(tx_script_malicious), None);

    let block_ref = data_store_malicious_account.block_header.block_num();
    let note_ids = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_2 = executor_2.execute_transaction(
        malicious_account_id,
        block_ref,
        &note_ids,
        Some(tx_args_malicious),
    );

    // Check that we got the expected result - TransactionExecutorError
    assert!(executed_transaction_2.is_err());
}

/// We test the Pay to script with 2 assets to test the loop inside the script.
/// So we create a note containing two assets that can only be consumed by the target account.
#[test]
fn p2id_script_multiple_assets() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id, 123).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 456).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_keypair_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_p2id_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset_1, fungible_asset_2],
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_keypair_felt)],
            vec![],
        )
        .unwrap();

    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, Some(tx_args_target))
        .unwrap();

    // vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset_1, fungible_asset_2]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let (malicious_pub_key, malicious_keypair_felt) = get_new_key_pair_with_advice_map();
    let malicious_account =
        get_account_with_default_account_code(malicious_account_id, malicious_pub_key, None);

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]));
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());
    executor_2.load_account(malicious_account_id).unwrap();
    let tx_script_malicious = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(malicious_pub_key, malicious_keypair_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_malicious = TransactionArgs::new(Some(tx_script_malicious), None);

    let block_ref = data_store_malicious_account.block_header.block_num();
    let note_origins = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_2 = executor_2.execute_transaction(
        malicious_account_id,
        block_ref,
        &note_origins,
        Some(tx_args_malicious),
    );

    // Check that we got the expected result - TransactionExecutorError
    assert!(executed_transaction_2.is_err());
}
