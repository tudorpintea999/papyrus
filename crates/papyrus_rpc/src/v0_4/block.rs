use jsonrpsee::types::ErrorObjectOwned;
use papyrus_storage::db::TransactionKind;
use papyrus_storage::header::HeaderStorageReader;
use papyrus_storage::StorageTxn;
use serde::{Deserialize, Serialize};
use starknet_api::block::{BlockHash, BlockNumber, BlockStatus, BlockTimestamp};
use starknet_api::core::{ContractAddress, GlobalRoot};

use super::error::BLOCK_NOT_FOUND;
use super::transaction::Transactions;
use crate::api::{BlockHashOrNumber, BlockId, Tag};
use crate::{get_latest_block_number, internal_server_error};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
pub struct BlockHeader {
    pub block_hash: BlockHash,
    pub parent_hash: BlockHash,
    pub block_number: BlockNumber,
    pub sequencer_address: ContractAddress,
    pub new_root: GlobalRoot,
    pub timestamp: BlockTimestamp,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct PendingBlockHeader {
    pub parent_hash: BlockHash,
    pub sequencer_address: ContractAddress,
    pub timestamp: BlockTimestamp,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
#[serde(untagged)]
pub enum GeneralBlockHeader {
    BlockHeader(BlockHeader),
    PendingBlockHeader(PendingBlockHeader),
}

impl From<starknet_api::block::BlockHeader> for BlockHeader {
    fn from(header: starknet_api::block::BlockHeader) -> Self {
        BlockHeader {
            block_hash: header.block_hash,
            parent_hash: header.parent_hash,
            block_number: header.block_number,
            sequencer_address: header.sequencer,
            new_root: header.state_root,
            timestamp: header.timestamp,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
pub struct Block {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BlockStatus>,
    #[serde(flatten)]
    pub header: GeneralBlockHeader,
    pub transactions: Transactions,
}

pub fn get_block_header_by_number<
    Mode: TransactionKind,
    BlockHeader: From<starknet_api::block::BlockHeader>,
>(
    txn: &StorageTxn<'_, Mode>,
    block_number: BlockNumber,
) -> Result<BlockHeader, ErrorObjectOwned> {
    let header = txn
        .get_block_header(block_number)
        .map_err(internal_server_error)?
        .ok_or_else(|| ErrorObjectOwned::from(BLOCK_NOT_FOUND))?;

    Ok(BlockHeader::from(header))
}

/// Return the closest block number that corresponds to the given block id and is accepted (i.e not
/// pending)
pub(crate) fn get_accepted_block_number<Mode: TransactionKind>(
    txn: &StorageTxn<'_, Mode>,
    block_id: BlockId,
) -> Result<BlockNumber, ErrorObjectOwned> {
    Ok(match block_id {
        BlockId::HashOrNumber(BlockHashOrNumber::Hash(block_hash)) => txn
            .get_block_number_by_hash(&block_hash)
            .map_err(internal_server_error)?
            .ok_or_else(|| ErrorObjectOwned::from(BLOCK_NOT_FOUND))?,
        BlockId::HashOrNumber(BlockHashOrNumber::Number(block_number)) => {
            // Check that the block exists.
            let last_block_number = get_latest_block_number(txn)?
                .ok_or_else(|| ErrorObjectOwned::from(BLOCK_NOT_FOUND))?;
            if block_number > last_block_number {
                return Err(ErrorObjectOwned::from(BLOCK_NOT_FOUND));
            }
            block_number
        }
        BlockId::Tag(Tag::Latest | Tag::Pending) => {
            get_latest_block_number(txn)?.ok_or_else(|| ErrorObjectOwned::from(BLOCK_NOT_FOUND))?
        }
    })
}
