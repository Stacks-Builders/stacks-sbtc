use rusqlite::{Connection as RusqliteConnection, Error as RusqliteError, Row as SqliteRow};
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use blockstack_lib::burnchains::Txid;
use blockstack_lib::types::chainstate::BurnchainHeaderHash;
use blockstack_lib::util::HexError;

use crate::peg_queue::{Error as PegQueueError, PegQueue, SbtcOp};
use crate::stacks_node::{Error as StacksNodeError, PegInOp, PegOutRequestOp, StacksNode};

use tracing::{debug, info};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Rusqlite Error: {0}")]
    RusqliteError(#[from] RusqliteError),
    #[error("JSON serialization failure: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Hex codec error: {0}")]
    HexError(#[from] HexError),
    #[error("Did not recognize status: {0}")]
    InvalidStatusError(String),
}

// Workaround to allow non-perfect conversions in `Entry::from_row`
impl From<Error> for rusqlite::Error {
    fn from(err: Error) -> Self {
        Self::InvalidColumnType(0, err.to_string(), rusqlite::types::Type::Text)
    }
}

pub struct SqlitePegQueue {
    conn: rusqlite::Connection,
    start_block_height: u64,
}

impl SqlitePegQueue {
    pub fn new<P: AsRef<Path>>(path: P, start_block_height: u64) -> Result<Self, Error> {
        Self::from_connection(RusqliteConnection::open(path)?, start_block_height)
    }

    pub fn in_memory(start_block_height: u64) -> Result<Self, Error> {
        Self::from_connection(RusqliteConnection::open_in_memory()?, start_block_height)
    }

    fn from_connection(conn: RusqliteConnection, start_block_height: u64) -> Result<Self, Error> {
        let this = Self {
            conn,
            start_block_height,
        };
        this.conn
            .execute(Self::create_sbtc_ops_table(), rusqlite::params![])?;
        this.conn
            .execute(Self::create_metadata_table(), rusqlite::params![])?;
        Ok(this)
    }

    fn poll_peg_in_ops<N: StacksNode>(
        &self,
        stacks_node: &N,
        block_height: u64,
    ) -> Result<(), PegQueueError> {
        match stacks_node.get_peg_in_ops(block_height) {
            Err(StacksNodeError::UnknownBlockHeight(height)) => {
                debug!("Failed to find burn block height {}", height);
            }
            Err(e) => return Err(PegQueueError::from(e)),
            Ok(peg_in_ops) => {
                for peg_in_op in peg_in_ops {
                    let entry = Entry::from(peg_in_op);
                    self.insert(&entry)?;
                }
            }
        }
        Ok(())
    }

    fn poll_peg_out_request_ops<N: StacksNode>(
        &self,
        stacks_node: &N,
        block_height: u64,
    ) -> Result<(), PegQueueError> {
        match stacks_node.get_peg_out_request_ops(block_height) {
            Err(StacksNodeError::UnknownBlockHeight(height)) => {
                debug!("Failed to find burn block height {}", height);
            }
            Err(e) => return Err(PegQueueError::from(e)),
            Ok(peg_out_request_ops) => {
                for peg_out_request_op in peg_out_request_ops {
                    let entry = Entry::from(peg_out_request_op);
                    self.insert(&entry)?;
                }
            }
        }
        Ok(())
    }
    fn insert(&self, entry: &Entry) -> Result<(), Error> {
        self.conn.execute(
            Self::sql_insert(),
            rusqlite::params![
                entry.txid.to_hex(),
                entry.burn_header_hash.to_hex(),
                entry.block_height as i64, // Stacks will crash before the coordinator if this is invalid
                serde_json::to_string(&entry.op)?,
                entry.status.as_str(),
            ],
        )?;

        Ok(())
    }

    fn get_single_entry_with_status(&self, status: &Status) -> Result<Option<Entry>, Error> {
        Ok(self
            .conn
            .prepare(Self::sql_select_status())?
            .query_map(rusqlite::params![status.as_str()], Entry::from_row)?
            .next()
            .transpose()?)
    }

    fn get_entry(
        &self,
        txid: &Txid,
        burn_header_hash: &BurnchainHeaderHash,
    ) -> Result<Entry, Error> {
        Ok(self.conn.prepare(Self::sql_select_pk())?.query_row(
            rusqlite::params![txid.to_hex(), burn_header_hash.to_hex()],
            Entry::from_row,
        )?)
    }

    fn last_processed_block_height(&self) -> Result<u64, Error> {
        Ok(self
            .conn
            .query_row(
                Self::sql_select_last_processed_block_height(),
                rusqlite::params![],
                |row| row.get::<_, i64>(0),
            )
            .map(|height| height as u64)?)
    }

    fn insert_last_processed_block_height(&self, height: u64) -> Result<(), Error> {
        self.conn.execute(
            Self::sql_insert_last_processed_block_height(),
            rusqlite::params![height as i64],
        )?;

        Ok(())
    }

    const fn create_sbtc_ops_table() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS sbtc_ops (
            txid TEXT NOT NULL,
            burn_header_hash TEXT NOT NULL,
            block_height INTEGER NOT NULL,
            op TEXT NOT NULL,
            status TEXT NOT NULL,

            PRIMARY KEY(txid, burn_header_hash)
        )
        "#
    }

    const fn create_metadata_table() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS peg_queue_metadata (
            id TEXT NOT NULL,

            last_processed_block_height INTEGER NOT NULL,

            PRIMARY KEY(id)
        )
        "#
    }

    const fn sql_insert() -> &'static str {
        r#"
        REPLACE INTO sbtc_ops (txid, burn_header_hash, block_height, op, status) VALUES (?1, ?2, ?3, ?4, ?5)
        "#
    }

    const fn sql_select_status() -> &'static str {
        r#"
        SELECT txid, burn_header_hash, block_height, op, status FROM sbtc_ops WHERE status=?1 ORDER BY block_height, op ASC
        "#
    }

    const fn sql_select_pk() -> &'static str {
        r#"
        SELECT txid, burn_header_hash, block_height, op, status FROM sbtc_ops WHERE txid=?1 AND burn_header_hash=?2
        "#
    }

    const fn sql_select_last_processed_block_height() -> &'static str {
        r#"
            SELECT last_processed_block_height FROM peg_queue_metadata WHERE id='peg_queue'
        "#
    }

    const fn sql_insert_last_processed_block_height() -> &'static str {
        r#"
            REPLACE INTO peg_queue_metadata (id, last_processed_block_height) VALUES ('peg_queue', ?1)
        "#
    }
}

impl PegQueue for SqlitePegQueue {
    fn sbtc_op(&self) -> Result<Option<SbtcOp>, PegQueueError> {
        let maybe_entry = self.get_single_entry_with_status(&Status::New)?;

        let Some(mut entry) = maybe_entry else {
            return Ok(None)
        };

        entry.status = Status::Pending;
        self.insert(&entry)?;

        Ok(Some(entry.op))
    }

    fn poll<N: StacksNode>(&self, stacks_node: &N) -> Result<(), PegQueueError> {
        let target_block_height = stacks_node.burn_block_height()?;
        let start_block_height = self
            .last_processed_block_height()
            .map(|count| count + 1)
            .unwrap_or(self.start_block_height);

        info!(
            "Checking for peg-in and peg-out requests for block heights {} to {}",
            start_block_height, target_block_height
        );

        let mut timestamp = Instant::now();

        for block_height in start_block_height..=target_block_height {
            self.poll_peg_in_ops(stacks_node, block_height)?;
            self.poll_peg_out_request_ops(stacks_node, block_height)?;
            self.insert_last_processed_block_height(block_height)?;

            if timestamp.elapsed().as_secs_f64() > 5.0 {
                info!("Processed block height {}", block_height);
                timestamp = Instant::now();
            }
        }

        Ok(())
    }

    fn acknowledge(
        &self,
        txid: &Txid,
        burn_header_hash: &BurnchainHeaderHash,
    ) -> Result<(), PegQueueError> {
        let mut entry = self.get_entry(txid, burn_header_hash)?;

        entry.status = Status::Acknowledged;
        self.insert(&entry)?;

        Ok(())
    }
}

#[derive(Debug)]
struct Entry {
    burn_header_hash: BurnchainHeaderHash,
    txid: Txid,
    block_height: u64,
    op: SbtcOp,
    status: Status,
}

impl Entry {
    fn from_row(row: &SqliteRow) -> Result<Self, RusqliteError> {
        let txid = Txid::from_hex(&row.get::<_, String>(0)?).map_err(Error::from)?;

        let burn_header_hash =
            BurnchainHeaderHash::from_hex(&row.get::<_, String>(1)?).map_err(Error::from)?;

        let block_height = row.get::<_, i64>(2)? as u64; // Stacks will crash before the coordinator if this is invalid

        let op: SbtcOp = serde_json::from_str(&row.get::<_, String>(3)?).map_err(Error::from)?;

        let status: Status = row.get::<_, String>(4)?.parse()?;

        Ok(Self {
            burn_header_hash,
            txid,
            block_height,
            op,
            status,
        })
    }
}

impl From<PegInOp> for Entry {
    fn from(op: PegInOp) -> Self {
        Self {
            block_height: op.block_height,
            status: Status::New,
            txid: op.txid,
            burn_header_hash: op.burn_header_hash,
            op: SbtcOp::PegIn(op),
        }
    }
}

impl From<PegOutRequestOp> for Entry {
    fn from(op: PegOutRequestOp) -> Self {
        Self {
            block_height: op.block_height,
            status: Status::New,
            txid: op.txid,
            burn_header_hash: op.burn_header_hash,
            op: SbtcOp::PegOutRequest(op),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Status {
    New,
    Pending,
    Acknowledged,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Pending => "pending",
            Self::Acknowledged => "acknowledged",
        }
    }
}

impl FromStr for Status {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(match s {
            "new" => Self::New,
            "pending" => Self::Pending,
            "acknowledged" => Self::Acknowledged,
            other => return Err(Error::InvalidStatusError(other.to_owned())),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::stacks_node;

    use blockstack_lib::{
        chainstate::stacks::address::PoxAddress,
        types::chainstate::StacksAddress,
        util::{hash::Hash160, secp256k1::MessageSignature},
    };
    use std::{collections::hash_map::DefaultHasher, hash::Hasher};

    use crate::peg_queue::PegQueue;

    use super::*;

    #[test]
    fn calling_sbtc_op_should_return_new_peg_ops() {
        let peg_queue = SqlitePegQueue::in_memory(1).unwrap();
        let number_of_simulated_blocks: u64 = 3;

        let stacks_node_mock = default_stacks_node_mock(number_of_simulated_blocks);

        // No ops before polling
        assert!(peg_queue.sbtc_op().unwrap().is_none());

        // Should cause the peg_queue to fetch 3 peg in ops
        peg_queue.poll(&stacks_node_mock).unwrap();

        for height in 1..=number_of_simulated_blocks {
            let next_op = peg_queue.sbtc_op().unwrap().unwrap();
            assert!(next_op.as_peg_in().is_some());
            assert_eq!(next_op.as_peg_in().unwrap().block_height, height);

            let next_op = peg_queue.sbtc_op().unwrap().unwrap();
            assert!(next_op.as_peg_out_request().is_some());
            assert_eq!(next_op.as_peg_out_request().unwrap().block_height, height);
        }
    }

    #[test]
    fn calling_poll_should_not_query_new_ops_if_at_block_height() {
        let peg_queue = SqlitePegQueue::in_memory(1).unwrap();
        let number_of_simulated_blocks: u64 = 3;

        let stacks_node_mock = default_stacks_node_mock(number_of_simulated_blocks);

        // Fast forward past first poll
        peg_queue.poll(&stacks_node_mock).unwrap();
        for _ in 1..=number_of_simulated_blocks {
            peg_queue.sbtc_op().unwrap().unwrap();
            peg_queue.sbtc_op().unwrap().unwrap();
        }

        let mut stacks_node_mock = stacks_node::MockStacksNode::new();

        stacks_node_mock
            .expect_burn_block_height()
            .returning(move || Ok(number_of_simulated_blocks));

        stacks_node_mock.expect_get_peg_in_ops().never();
        stacks_node_mock.expect_get_peg_out_request_ops().never();

        peg_queue.poll(&stacks_node_mock).unwrap();
    }

    #[test]
    fn calling_poll_should_find_new_ops_if_at_new_block_height() {
        let peg_queue = SqlitePegQueue::in_memory(1).unwrap();
        let number_of_simulated_blocks: u64 = 3;
        let number_of_simulated_blocks_second_poll: u64 = 5;

        let stacks_node_mock = default_stacks_node_mock(number_of_simulated_blocks);

        // Fast forward past first poll
        peg_queue.poll(&stacks_node_mock).unwrap();
        for _ in 1..=number_of_simulated_blocks {
            peg_queue.sbtc_op().unwrap().unwrap();
            peg_queue.sbtc_op().unwrap().unwrap();
        }

        let stacks_node_mock = default_stacks_node_mock(number_of_simulated_blocks_second_poll);
        peg_queue.poll(&stacks_node_mock).unwrap();

        for height in number_of_simulated_blocks + 1..=number_of_simulated_blocks_second_poll {
            let next_op = peg_queue.sbtc_op().unwrap().unwrap();
            assert!(next_op.as_peg_in().is_some());
            assert_eq!(next_op.as_peg_in().unwrap().block_height, height);

            let next_op = peg_queue.sbtc_op().unwrap().unwrap();
            assert!(next_op.as_peg_out_request().is_some());
            assert_eq!(next_op.as_peg_out_request().unwrap().block_height, height);
        }
    }

    #[test]
    fn acknowledged_entries_should_have_acknowledge_status() {
        let peg_queue = SqlitePegQueue::in_memory(1).unwrap();
        let number_of_simulated_blocks: u64 = 1;

        let stacks_node_mock = default_stacks_node_mock(number_of_simulated_blocks);
        peg_queue.poll(&stacks_node_mock).unwrap();

        let next_op = peg_queue.sbtc_op().unwrap().unwrap();
        let peg_in_op = next_op.as_peg_in().unwrap();
        peg_queue
            .acknowledge(&peg_in_op.txid, &peg_in_op.burn_header_hash)
            .unwrap();

        let entry = peg_queue
            .get_entry(&peg_in_op.txid, &peg_in_op.burn_header_hash)
            .unwrap();

        assert_eq!(entry.status, Status::Acknowledged);
    }

    #[test]
    fn should_start_at_last_observed_block_height_when_polling() {
        let start_block_height: u64 = 10;
        let initial_node_block_height: u64 = 20;
        let second_poll_node_block_height: u64 = 40;

        let peg_queue = SqlitePegQueue::in_memory(start_block_height).unwrap();

        let stacks_node_mock = default_stacks_node_mock(initial_node_block_height);
        peg_queue.poll(&stacks_node_mock).unwrap();

        assert_eq!(peg_queue.last_processed_block_height().unwrap(), 20);

        let stacks_node_mock = stacks_node_mock_with_no_sbtc_ops(second_poll_node_block_height);
        peg_queue.poll(&stacks_node_mock).unwrap();

        assert_eq!(peg_queue.last_processed_block_height().unwrap(), 40);
    }

    fn default_stacks_node_mock(block_height: u64) -> stacks_node::MockStacksNode {
        let mut stacks_node_mock = stacks_node::MockStacksNode::new();

        stacks_node_mock
            .expect_burn_block_height()
            .returning(move || Ok(block_height));

        stacks_node_mock
            .expect_get_peg_in_ops()
            .returning(|height| Ok(vec![peg_in_op(height)]));

        stacks_node_mock
            .expect_get_peg_out_request_ops()
            .returning(|height| Ok(vec![peg_out_request_op(height)]));

        stacks_node_mock
    }

    fn stacks_node_mock_with_no_sbtc_ops(block_height: u64) -> stacks_node::MockStacksNode {
        let mut stacks_node_mock = stacks_node::MockStacksNode::new();

        stacks_node_mock
            .expect_burn_block_height()
            .returning(move || Ok(block_height));

        stacks_node_mock
            .expect_get_peg_in_ops()
            .returning(|_height| Ok(vec![]));

        stacks_node_mock
            .expect_get_peg_out_request_ops()
            .returning(|_height| Ok(vec![]));

        stacks_node_mock
    }

    fn peg_in_op(block_height: u64) -> PegInOp {
        let recipient_stx_addr = StacksAddress::new(26, Hash160([0; 20]));
        let peg_wallet_address =
            PoxAddress::Standard(StacksAddress::new(0, Hash160([0; 20])), None);

        PegInOp {
            recipient: recipient_stx_addr.into(),
            peg_wallet_address,
            amount: 1337,
            memo: vec![1, 3, 3, 7],
            txid: Txid(hash_and_expand(block_height, 1)),
            burn_header_hash: BurnchainHeaderHash(hash_and_expand(block_height, 0)),
            block_height,
            vtxindex: 0,
        }
    }

    fn peg_out_request_op(block_height: u64) -> PegOutRequestOp {
        let recipient_stx_addr = StacksAddress::new(26, Hash160([0; 20]));
        let peg_wallet_address =
            PoxAddress::Standard(StacksAddress::new(0, Hash160([0; 20])), None);

        stacks_node::PegOutRequestOp {
            recipient: PoxAddress::Standard(recipient_stx_addr, None),
            peg_wallet_address,
            amount: 1337,
            fulfillment_fee: 1000,
            signature: MessageSignature([0; 65]),
            memo: vec![1, 3, 3, 7],
            txid: Txid(hash_and_expand(block_height, 2)),
            burn_header_hash: BurnchainHeaderHash(hash_and_expand(block_height, 0)),
            block_height,
            vtxindex: 0,
        }
    }

    fn hash_and_expand(val: u64, nonce: u64) -> [u8; 32] {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(val);
        hasher.write_u64(nonce);
        let hash = hasher.finish();

        hash.to_be_bytes().repeat(4).try_into().unwrap()
    }
}
