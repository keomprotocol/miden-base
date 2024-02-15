use miden_lib::notes::{create_limit_swap_note, utils::build_p2id_recipient, utils::build_partial_recipient};
use miden_objects::{
    accounts::{Account, AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    crypto::rand::RpoRandomCoin,
    notes::{NoteAssets, NoteMetadata},
    transaction::OutputNote,
    Felt,
};
use miden_tx::TransactionExecutor;
use mock::constants::{
    ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    ACCOUNT_ID_SENDER, DEFAULT_AUTH_SCRIPT,
};

use crate::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map,
    prove_and_verify_transaction, MockDataStore,
};

#[test]
fn prove_limit_swap_script() {
    // Create assets
    let faucet_id_A = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_A: Asset = FungibleAsset::new(faucet_id_A, 50).unwrap().into();

    let faucet_id_B = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_B: Asset = FungibleAsset::new(faucet_id_B, 100).unwrap().into();
/*
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let non_fungible_asset: Asset = NonFungibleAsset::new(
        &NonFungibleAssetDetails::new(faucet_id_2, vec![1, 2, 3, 4]).unwrap(),
    )
    .unwrap()
    .into();*/

    // Create sender and target account
    let alice_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let bob_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (bob_pub_key, bob_sk_felt) = get_new_key_pair_with_advice_map();
    let bob_account = get_account_with_default_account_code(
        bob_account_id,
        bob_pub_key,
        Some(fungible_asset_B),
    );

    // Create the note containing the LIMIT_SWAP script
    // Alice offers 50 token_A for 100 token_B
    let (note, payback_serial_num, note_serial_num) = create_limit_swap_note(
        alice_account_id,
        fungible_asset_A,
        fungible_asset_B,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(bob_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(bob_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let tx_script_target = executor
        .compile_tx_script(tx_script_code.clone(), vec![(bob_pub_key, bob_sk_felt)], vec![])
        .unwrap();

    let amount_to_send:    i32 = 50;
    let amount_to_consume: i32 = 25;

    let mut note_args_map = BTreeMap::new();
    let bobs_args : Word  = [ZERO, ZERO, amount_to_send, amount_to_consume]; 
    note_args_map.insert(note.note_id(), bobs_args);

    // Execute the transaction where Bob attempts to consume half the amount offered in Alice's note
    let transaction_result = executor
        .execute_transaction(bob_account_id, block_ref, &note_ids, Some(tx_script_target), Some(note_args_map))
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(transaction_result.clone()).is_ok());

    // Bob account vault delta
    let bob_account_after: Account = Account::new(
        bob_account.id(),
        AssetVault::new(&[fungible_asset_A]).unwrap(),
        bob_account.storage().clone(),
        bob_account.code().clone(),
        Felt::new(2),
    );

    // Check that the target account has received the asset from the note
    assert_eq!(transaction_result.final_account().hash(), bob_account_after.hash());

    // Check that two `Note`s have been created (p2id and limit_swap clone)
    assert_eq!(transaction_result.output_notes().num_notes(), 2);

    // Check if the created `P2ID Note` is what we expect

    let p2id_recipient                   = build_p2id_recipient(alice_account_id, payback_serial_num).unwrap();
    let p2id_metadata                    = NoteMetadata::new(bob_account_id, alice_account_id.into());
    let sent_fungible_asset_B:     Asset = FungibleAsset::new(faucet_id_B, amount_to_send).unwrap().into();
    let consumed_fungible_asset_A: Asset = FungibleAsset::new(faucet_id_A, amount_to_consume).unwrap().into();

    let p2id_assets                      = NoteAssets::new(&[sent_fungible_asset_B]).unwrap();
    let p2id_expected_note               = OutputNote::new(p2id_recipient, p2id_assets, p2id_metadata);
    let p2id_created_note                = transaction_result.output_notes().get_note(0);

    assert_eq!(created_note, &expected_note);

     // Check if the created `LIMIT_SWAP clone Note` is what we expect

    let limit_swap_created_note          = transaction_result.output_notes().get_note(1);

    let bytes                        = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/LIMIT_SWAP.masb"));
    let note_script                  = build_note_script(bytes)?;
    let limit_swap_recipient         = build_partial_recipient(note_script, note_serial_num)
}
