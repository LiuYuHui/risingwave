// Copyright 2023 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{fmt, vec};

use risingwave_common::catalog::{Field, Schema};
use risingwave_common::error::Result;
use risingwave_common::types::DataType;

use super::{
    gen_filter_and_pushdown, BatchDelete, ColPrunable, ExprRewritable, PlanBase, PlanRef,
    PlanTreeNodeUnary, PredicatePushdown, ToBatch, ToStream,
};
use crate::catalog::TableId;
use crate::optimizer::plan_node::{
    ColumnPruningContext, PredicatePushdownContext, RewriteStreamContext, ToStreamContext,
};
use crate::optimizer::property::FunctionalDependencySet;
use crate::utils::{ColIndexMapping, Condition};

/// [`LogicalDelete`] iterates on input relation and delete the data from specified table.
///
/// It corresponds to the `DELETE` statements in SQL.
#[derive(Debug, Clone)]
pub struct LogicalDelete {
    pub base: PlanBase,
    table_name: String, // explain-only
    table_id: TableId,
    input: PlanRef,
    returning: bool,
}

impl LogicalDelete {
    /// Create a [`LogicalDelete`] node. Used internally by optimizer.
    pub fn new(input: PlanRef, table_name: String, table_id: TableId, returning: bool) -> Self {
        let ctx = input.ctx();
        let schema = if returning {
            input.schema().clone()
        } else {
            Schema::new(vec![Field::unnamed(DataType::Int64)])
        };
        let fd_set = FunctionalDependencySet::new(schema.len());
        let base = PlanBase::new_logical(ctx, schema, vec![], fd_set);
        Self {
            base,
            table_name,
            table_id,
            input,
            returning,
        }
    }

    /// Create a [`LogicalDelete`] node. Used by planner.
    pub fn create(
        input: PlanRef,
        table_name: String,
        table_id: TableId,
        returning: bool,
    ) -> Result<Self> {
        Ok(Self::new(input, table_name, table_id, returning))
    }

    pub(super) fn fmt_with_name(&self, f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
        write!(
            f,
            "{} {{ table: {}{} }}",
            name,
            self.table_name,
            if self.returning {
                ", returning: true"
            } else {
                ""
            }
        )
    }

    #[must_use]
    pub fn table_id(&self) -> TableId {
        self.table_id
    }

    pub fn has_returning(&self) -> bool {
        self.returning
    }
}

impl PlanTreeNodeUnary for LogicalDelete {
    fn input(&self) -> PlanRef {
        self.input.clone()
    }

    fn clone_with_input(&self, input: PlanRef) -> Self {
        Self::new(
            input,
            self.table_name.clone(),
            self.table_id,
            self.returning,
        )
    }
}

impl_plan_tree_node_for_unary! { LogicalDelete }

impl fmt::Display for LogicalDelete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_name(f, "LogicalDelete")
    }
}

impl ColPrunable for LogicalDelete {
    fn prune_col(&self, _required_cols: &[usize], ctx: &mut ColumnPruningContext) -> PlanRef {
        let required_cols: Vec<_> = (0..self.input.schema().len()).collect();
        self.clone_with_input(self.input.prune_col(&required_cols, ctx))
            .into()
    }
}

impl ExprRewritable for LogicalDelete {}

impl PredicatePushdown for LogicalDelete {
    fn predicate_pushdown(
        &self,
        predicate: Condition,
        ctx: &mut PredicatePushdownContext,
    ) -> PlanRef {
        gen_filter_and_pushdown(self, predicate, Condition::true_cond(), ctx)
    }
}

impl ToBatch for LogicalDelete {
    fn to_batch(&self) -> Result<PlanRef> {
        let new_input = self.input().to_batch()?;
        let new_logical = self.clone_with_input(new_input);
        Ok(BatchDelete::new(new_logical).into())
    }
}

impl ToStream for LogicalDelete {
    fn to_stream(&self, _ctx: &mut ToStreamContext) -> Result<PlanRef> {
        unreachable!("delete should always be converted to batch plan");
    }

    fn logical_rewrite_for_stream(
        &self,
        _ctx: &mut RewriteStreamContext,
    ) -> Result<(PlanRef, ColIndexMapping)> {
        unreachable!("delete should always be converted to batch plan");
    }
}
