use core::fmt;

use miden_objects::{
    assembly::AssemblyError, notes::NoteId, Felt, NoteError, TransactionInputError,
    TransactionOutputError,
};
use miden_verifier::VerificationError;

use super::{AccountError, AccountId, Digest, ExecutionError};

// TRANSACTION COMPILER ERROR
// ================================================================================================

#[derive(Debug)]
pub enum TransactionCompilerError {
    AccountInterfaceNotFound(AccountId),
    BuildCodeBlockTableFailed(AssemblyError),
    CompileNoteScriptFailed(AssemblyError),
    CompileTxScriptFailed(AssemblyError),
    LoadAccountFailed(AccountError),
    NoteIncompatibleWithAccountInterface(Digest),
    NoteScriptError(NoteError),
    NoTransactionDriver,
    TxScriptIncompatibleWithAccountInterface(Digest),
}

impl fmt::Display for TransactionCompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionCompilerError {}

// TRANSACTION EXECUTOR ERROR
// ================================================================================================

#[derive(Debug)]
pub enum TransactionExecutorError {
    CompileNoteScriptFailed(TransactionCompilerError),
    CompileTransactionScriptFailed(TransactionCompilerError),
    CompileTransactionFailed(TransactionCompilerError),
    ExecuteTransactionProgramFailed(ExecutionError),
    FetchAccountCodeFailed(DataStoreError),
    FetchTransactionInputsFailed(DataStoreError),
    InconsistentAccountId {
        input_id: AccountId,
        output_id: AccountId,
    },
    InconsistentAccountNonceDelta {
        expected: Option<Felt>,
        actual: Option<Felt>,
    },
    InvalidTransactionOutput(TransactionOutputError),
    LoadAccountFailed(TransactionCompilerError),
}

impl fmt::Display for TransactionExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionExecutorError {}

// TRANSACTION PROVER ERROR
// ================================================================================================

#[derive(Debug)]
pub enum TransactionProverError {
    ProveTransactionProgramFailed(ExecutionError),
    InvalidTransactionOutput(TransactionOutputError),
}

impl fmt::Display for TransactionProverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionProverError {}

// TRANSACTION VERIFIER ERROR
// ================================================================================================

#[derive(Debug)]
pub enum TransactionVerifierError {
    TransactionVerificationFailed(VerificationError),
    InsufficientProofSecurityLevel(u32, u32),
}

impl fmt::Display for TransactionVerifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionVerifierError {}

// DATA STORE ERROR
// ================================================================================================

#[derive(Debug)]
pub enum DataStoreError {
    AccountNotFound(AccountId),
    BlockNotFound(u32),
    InvalidTransactionInput(TransactionInputError),
    InternalError(String),
    NoteNotFound(NoteId),
}

impl fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DataStoreError {}
