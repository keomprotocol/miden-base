# Transaction Contexts
Miden assembly program execution can span multiple isolated contexts. An execution context defines its own memory space which is not accessible from other execution contexts. Note scripts cannot directly write into account data. This should be possible if only if the account exposes respective functions.

The kernel program always starts executing in a root context. Thus, the prologue sets the memory for the root context. To move execution into a different context, we can invoke a procedure using the `call` or `dyncall` instruction. In fact, any time we invoke a procedure using the `call` instruction, the procedure is executed in a new context. 

While executing in a note, account, or tx script context, we can request to execute some procedures in the kernel context, which is where all necessary information was stored during the prologue. Switching to the kernle context can be done via the `syscall` instruction. The set of procedures which can be invoked via the `syscall` instruction is limited by the [transaction kernel API](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/kernels/transaction/api.masm). Once the procedure call via `syscall` returns, the execution moves back to the note, account, or tx script from which it was invoked.

\
\


<p align="center">
    <img src="../diagrams/architecture/transaction/Transaction_contexts.png" style="width: 100%;">
</p>

\
\

The above diagram shows different context switches in a simple transaction. In this example, an account consumes a [P2ID](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/note_scripts/P2ID.masm) note and receives the asset into its vault. As with in any MASM program, the transaction kernel program starts in the root context. It executes the Prologue and stores all necessary information into the root memory. 

The next step, note processing, starts with a `dyncall` and in doing so, invoking the note script. This command moves execution into a different context **(1)**. In this new context, the note has no access to the kernel memory. After a successful ID check, which changes back to the kernel context twice to get the note inputs and the account id, the script executes the `add_note_assets_to_account` procedure.

```
# Pay-to-ID script: adds all assets from the note to the account, assuming ID of the account
# matches target account ID specified by the note inputs.
# ...
begin
    
    ... <check correct ID>

    exec.add_note_assets_to_account
    # => [...]
end
```

The procedure cannot simply add assets to the account, because it is executed in a note context. Therefore, it needs to `call` the account interface. And in doing so, it moves execution into a second context - Account context - isolated from the note context **(2)**. 

```
#! Helper procedure to add all assets of a note to an account.
#! ...
proc.add_note_assets_to_account
    ...

    while.true
        ...

        # load the asset and add it to the account
        mem_loadw call.wallet::receive_asset
        # => [ASSET, ptr, end_ptr, ...]
        ...
    end
    ...
end
```

The [wallet](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/miden/contracts/wallets/basic.masm) smart contract provides an interface for accounts to recieve and send assets. In this new context, the wallet calls the `add_asset` procedure of the account API. 

```
export.receive_asset
    exec.account::add_asset
    ...
end
```

The [account API](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/miden/account.masm#L162) exposes procedures to manage accounts. This particular procedure that was called by the wallet invokes a `syscall` to return back to the root context **(3)**, where the account vault is stored in memory (see Prologue). `syscall` can incoke all procedures defined in the [Kernel API](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/kernels/transaction/api.masm).

```
#! Add the specified asset to the vault.
#! ...
export.add_asset
    syscall.account_vault_add_asset
end
```

Now, the asset can be safely added to the vault within the kernel context and the note successfully be processed. 

# Transaction Procedures
There are user-facing procedures and kernel procedures. Users don't directly invoke kernel procedures, but indirectly via account code, note or transaction scripts. In that case, kernel procedures can only be invoked by a `syscall` instruction which always executes in the kernel context. 

## User-facing Procedures (APIs)
These procedures can be used to create smart contract / account code, note scripts or account scripts. They basically serve as an API for the underlying kernel procedures. If a procedure can be called in the current context an `exec` is sufficient, otherwise if being the wrong context procedures must be invoked by `call`. Users will never need to invoke `syscall` procedures themselves. 

_Note: If capitalized, a variable represents a `Word`, e.g., `ACCT_HASH` consists of four `Felts`. If lowercase, the variable is represented by a single `Felt`._

### Account
To import the account procedures set `use.miden::account` at the beginning of the file. Any procedure that changes the account state, can only be invoked in the account context and not by note or transaction scripts. All procedures invoke `syscall` to the kernel API and some are restricted by the kernel procedure `exec.authenticate_account_origin`, which fails if the parent context is not the executing account.

| Procedure name            | Stack      | Output       | Context | Description                                                         |
|---------------------------|------------|--------------|---------|---------------------------------------------------------------------|
| `get_id`                  | `[]`       | `[acct_id]`  | account, note | <details><summary>View</summary>Returns the account id. acct_id is the account id.</details> |
| `get_nonce`               | `[]`       | `[nonce]`    | account, note | <details><summary>View</summary>Returns the account nonce. nonce is the account nonce.</details> |
| `get_initial_hash`        | `[]`       | `[H]`        | account, note | <details><summary>View</summary>Returns the initial account hash. H is the initial account hash.</details> |
| `get_current_hash`        | `[]`       | `[ACCT_HASH]`| account, note | <details><summary>View</summary>Computes and returns the account hash from account data stored in memory. ACCT_HASH is the hash of the account data.</details> |
| `incr_nonce`              | `[value]`  | `[]`         | account | <details><summary>View</summary>Increments the account nonce by the provided value. value is the value to increment the nonce by. value can be at most 2^32 - 1 otherwise this procedure panics.</details> |
| `get_item`                | `[index]`  | `[VALUE]`    | account, note | <details><summary>View</summary>Gets an item from the account storage. Panics if the index is out of bounds. index is the index of the item to get. VALUE is the value of the item.</details> |
| `set_item`                | `[index, V']` | `[R', V]` | account | <details><summary>View</summary>Sets an item in the account storage. Panics if the index is out of bounds. index is the index of the item to set. V' is the value to set. V is the previous value of the item. R' is the new storage root.</details> |
| `set_code`                | `[CODE_ROOT]`| `[]`       | account | <details><summary>View</summary>Sets the code of the account the transaction is being executed against. This procedure can only be executed on regular accounts with updatable code. Otherwise, this procedure fails. CODE_ROOT is the hash of the code to set.</details> |
| `get_balance`             | `[faucet_id]`| `[balance]`| account, note | <details><summary>View</summary>Returns the balance of a fungible asset associated with a faucet_id. Panics if the asset is not a fungible asset. faucet_id is the faucet id of the fungible asset of interest. balance is the vault balance of the fungible asset.</details> |
| `has_non_fungible_asset`  | `[ASSET]`   | `[has_asset]`| account, note | <details><summary>View</summary>Returns a boolean indicating whether the non-fungible asset is present in the vault. Panics if the ASSET is a fungible asset. ASSET is the non-fungible asset of interest. has_asset is a boolean indicating whether the account vault has the asset of interest.</details> |
| `add_asset`               | `[ASSET]`   | `[ASSET']`  | account | <details><summary>View</summary>Add the specified asset to the vault. Panics under various conditions. ASSET' final asset in the account vault defined as follows: If ASSET is a non-fungible asset, then ASSET' is the same as ASSET. If ASSET is a fungible asset, then ASSET' is the total fungible asset in the account vault after ASSET was added to it.</details> |
| `remove_asset`            | `[ASSET]`   | `[ASSET]`   | account | <details><summary>View</summary>Remove the specified asset from the vault. Panics under various conditions. ASSET is the asset to remove from the vault.</details> |
| `get_vault_commitment`    | `[]`        | `[COM]`     | account, note | <details><summary>View</summary>Returns a commitment to the account vault. COM is a commitment to the account vault.</details> |


### Note
To import the note procedures set `use.miden::note` at the beginning of the file. All procedures are restricted to the note context.

| Procedure name           | Inputs              | Outputs               | Context | Description                                                                                                                         |
|--------------------------|---------------------|-----------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------|
| `get_assets`             | `[dest_ptr]`        | `[num_assets, dest_ptr]` | note | <details><summary>View</summary>Writes the assets of the currently executing note into memory starting at the specified address. dest_ptr is the memory address to write the assets. num_assets is the number of assets in the currently executing note.</details> |
| `get_inputs`             | `[dest_ptr]`        | `[dest_ptr]`            | note | <details><summary>View</summary>Writes the inputs of the currently executed note into memory starting at the specified address. dest_ptr is the memory address to write the inputs.</details> |
| `get_sender`             | `[]`                | `[sender]`             | note | <details><summary>View</summary>Returns the sender of the note currently being processed. Panics if a note is not being processed. sender is the sender of the note currently being processed.</details> |


### Tx
To import the transaction procedures set `use.miden::tx` at the beginning of the file. Only the `create_note` procedure is restricted to the account context.

| Procedure name           | Inputs           | Outputs     | Context | Description                                                                                                                                                                  |
|--------------------------|------------------|-------------|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `get_block_number`       | `[]`             | `[num]`     | account, note | <details><summary>View</summary>Returns the block number of the last known block at the time of transaction execution. num is the last known block number.</details> |
| `get_block_hash`         | `[]`             | `[H]`       |  account, note | <details><summary>View</summary>Returns the block hash of the last known block at the time of transaction execution. H is the last known block hash.</details> |
| `get_input_notes_hash`   | `[]`             | `[COM]`     |  account, note | <details><summary>View</summary>Returns the input notes hash. This is computed as a sequential hash of (nullifier, script_root) tuples over all input notes. COM is the input notes hash.</details> |
| `get_output_notes_hash`  | `[0, 0, 0, 0]`   | `[COM]`     |  account, note | <details><summary>View</summary>Returns the output notes hash. This is computed as a sequential hash of (note_hash, note_metadata) tuples over all output notes. COM is the output notes hash.</details> |
| `create_note`            | `[ASSET, tag, RECIPIENT]` | `[ptr]` | account | <details><summary>View</summary>Creates a new note and returns a pointer to the memory address at which the note is stored. ASSET is the asset to be included in the note. tag is the tag to be included in the note. RECIPIENT is the recipient of the note. ptr is the pointer to the memory address at which the note is stored.</details> |


### Asset
To import the asset procedures set `use.miden::asset` at the beginning of the file. These procedures can only be called by faucet accounts.

| Procedure name               | Stack               | Output    | Context | Description                                                                                                                                                 |
|------------------------------|---------------------|-----------|---------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `build_fungible_asset`       | `[faucet_id, amount]` | `[ASSET]` | faucet | <details><summary>View</summary>Builds a fungible asset for the specified fungible faucet and amount. faucet_id is the faucet to create the asset for. amount is the amount of the asset to create. ASSET is the built fungible asset.</details> |
| `create_fungible_asset`      | `[amount]`          | `[ASSET]` | faucet | <details><summary>View</summary>Creates a fungible asset for the faucet the transaction is being executed against. amount is the amount of the asset to create. ASSET is the created fungible asset.</details> |
| `build_non_fungible_asset`   | `[faucet_id, DATA_HASH]` | `[ASSET]` | faucet | <details><summary>View</summary>Builds a non-fungible asset for the specified non-fungible faucet and DATA_HASH. faucet_id is the faucet to create the asset for. DATA_HASH is the data hash of the non-fungible asset to build. ASSET is the built non-fungible asset.</details> |
| `create_non_fungible_asset`  | `[DATA_HASH]`        | `[ASSET]` | faucet | <details><summary>View</summary>Creates a non-fungible asset for the faucet the transaction is being executed against. DATA_HASH is the data hash of the non-fungible asset to create. ASSET is the created non-fungible asset.</details> |

### Faucet
To import the faucet procedures set `use.miden::faucet` at the beginning of the file.

| Procedure name           | Stack      | Outputs           | Context | Description                                                                                                                                                                                                                     |
|--------------------------|------------|-------------------|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `mint`                   | `[ASSET]`  | `[ASSET]`         | faucet | <details><summary>View</summary>Mint an asset from the faucet the transaction is being executed against. Panics under various conditions. ASSET is the asset that was minted.</details> |
| `burn`                   | `[ASSET]`  | `[ASSET]`         | faucet | <details><summary>View</summary>Burn an asset from the faucet the transaction is being executed against. Panics under various conditions. ASSET is the asset that was burned.</details> |
| `get_total_issuance`     | `[]`       | `[total_issuance]`| faucet | <details><summary>View</summary>Returns the total issuance of the fungible faucet the transaction is being executed against. Panics if the transaction is not being executed against a fungible faucet. total_issuance is the total issuance of the fungible faucet the transaction is being executed against.</details> |


## Kernel Procedures 
WIP - we will add those later. 

