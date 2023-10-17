use crate::{assembler::assembler, auth::AuthScheme};
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountType, AccountVault},
    assembly::ModuleAst,
    crypto::merkle::MerkleStore,
    utils::{format, string::String, vec},
    AccountError, Word, ZERO,
};

/// Creates a new account with basic wallet interface and the specified authentication scheme.
///
/// The basic wallet interface exposes two procedures:
/// - `receive_asset`, which can be used to add an asset to the account.
/// - `send_asset`, which can be used to remove an asset from the account and put into a note
///    addressed to the specified recipient.
///
/// Both methods require authentication. The authentication procedure is defined by the specified
/// authentication scheme. Public key information for the scheme is stored in the account storage
/// at slot 0.
pub fn create_basic_wallet(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
) -> Result<(Account, Word), AccountError> {
    let (auth_scheme_procedure, storage_slot_0): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let account_code_string: String = format!(
        "
    use.miden::wallets::basic->basic_wallet
    use.miden::eoa::basic

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.{auth_scheme_procedure}
    
    "
    );
    let account_code_src: &str = &account_code_string;

    let account_code_ast = ModuleAst::parse(account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = assembler();
    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler)?;

    let account_storage = AccountStorage::new(vec![(0, storage_slot_0)], MerkleStore::new())?;
    let account_vault = AccountVault::new(&[])?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        AccountType::RegularAccountUpdatableCode,
        false,
        account_code.root(),
        account_storage.root(),
    )?;
    let account_id = AccountId::new(account_seed, account_code.root(), account_storage.root())?;
    Ok((
        Account::new(account_id, account_vault, account_storage, account_code, ZERO),
        account_seed,
    ))
}
