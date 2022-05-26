use std::sync::Arc;

use futures_async_stream::try_stream;
// Copyright 2022 Singularity Data
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use itertools::Itertools;
use risingwave_common::array::DataChunk;
use risingwave_common::catalog::{ColumnDesc, Schema, TableId};
use risingwave_common::error::{Result, RwError};
use risingwave_pb::batch_plan::plan_node::NodeBody;
use risingwave_storage::table::cell_based_table::CellBasedTable;
use risingwave_storage::{dispatch_state_store, Keyspace, StateStore, StateStoreImpl};

use crate::executor::monitor::BatchMetrics;
use crate::executor::{
    BoxedDataChunkStream, BoxedExecutor, BoxedExecutorBuilder, Executor, ExecutorBuilder,
};
use crate::task::BatchTaskContext;

/// Executor that scans data from row table
pub struct RowSeqScanExecutor<S: StateStore> {
    table: CellBasedTable<S>,
    primary: bool,
    chunk_size: usize,
    schema: Schema,
    identity: String,
    epoch: u64,
    stats: Arc<BatchMetrics>,
}

impl<S: StateStore> RowSeqScanExecutor<S> {
    pub fn new(
        table: CellBasedTable<S>,
        chunk_size: usize,
        primary: bool,
        identity: String,
        epoch: u64,
        stats: Arc<BatchMetrics>,
    ) -> Self {
        let schema = table.schema().clone();

        Self {
            table,
            primary,
            chunk_size,
            schema,
            identity,
            epoch,
            stats,
        }
    }

    // TODO: Remove this when we support real partition-scan.
    // For shared storage like Hummock, we are using a fake partition-scan now. If `self.primary` is
    // false, we'll ignore this scanning and yield no chunk.
    fn should_ignore(&self) -> bool {
        !self.primary
    }
}

pub struct RowSeqScanExecutorBuilder {}

impl RowSeqScanExecutorBuilder {
    // TODO: decide the chunk size for row seq scan
    pub const DEFAULT_CHUNK_SIZE: usize = 1024;
}

impl BoxedExecutorBuilder for RowSeqScanExecutorBuilder {
    fn new_boxed_executor<C: BatchTaskContext>(
        source: &ExecutorBuilder<C>,
    ) -> Result<BoxedExecutor> {
        let seq_scan_node = try_match_expand!(
            source.plan_node().get_node_body().unwrap(),
            NodeBody::RowSeqScan
        )?;

        let table_id = TableId {
            table_id: seq_scan_node.table_desc.as_ref().unwrap().table_id,
        };
        let column_descs = seq_scan_node
            .column_descs
            .iter()
            .map(|column_desc| ColumnDesc::from(column_desc.clone()))
            .collect_vec();
        dispatch_state_store!(
            source.batch_task_context().try_get_state_store()?,
            state_store,
            {
                let keyspace = Keyspace::table_root(state_store.clone(), &table_id);
                let storage_stats = state_store.stats();
                let batch_stats = source.batch_task_context().stats();
                let table = CellBasedTable::new_adhoc(keyspace, column_descs, storage_stats);
                Ok(Box::new(RowSeqScanExecutor::new(
                    table,
                    RowSeqScanExecutorBuilder::DEFAULT_CHUNK_SIZE,
                    source.task_id.task_id == 0,
                    source.plan_node().get_identity().clone(),
                    source.epoch(),
                    batch_stats,
                )))
            }
        )
    }
}

impl<S: StateStore> Executor for RowSeqScanExecutor<S> {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn identity(&self) -> &str {
        &self.identity
    }

    fn execute(self: Box<Self>) -> BoxedDataChunkStream {
        self.do_execute()
    }
}

impl<S: StateStore> RowSeqScanExecutor<S> {
    #[try_stream(boxed, ok = DataChunk, error = RwError)]
    async fn do_execute(self: Box<Self>) {
        if !self.should_ignore() {
            let mut iter = self.table.iter(self.epoch).await.map_err(RwError::from)?;

            loop {
                let timer = self.stats.row_seq_scan_next_duration.start_timer();

                let chunk = iter
                    .collect_data_chunk(&self.table, Some(self.chunk_size))
                    .await
                    .map_err(RwError::from)?;
                timer.observe_duration();

                if let Some(chunk) = chunk {
                    yield chunk
                } else {
                    break;
                }
            }
        }
    }
}