use crate::{
    helix_engine::{
        traversal_core::{traversal_value::TraversalValue, traversal_iter::RoTraversalIterator},
        storage_core::{HelixGraphStorage, storage_methods::StorageMethods},
        types::GraphError,
        traversal_core::ops::util::paths::ShortestPathIterator,
        traversal_core::ops::util::paths::PathType,
    },
    utils::{label_hash::hash_label},
};
use heed3::RoTxn;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

pub struct CyclePathIterator<'a, I> {
    inner: ShortestPathIterator<'a, I>,
}


impl<'a, I: Iterator<Item = Result<TraversalValue, GraphError>>> Iterator for CyclePathIterator<'a, I> {
    type Item = Result<TraversalValue, GraphError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Call ShortestPathIterator's next, but enforce is_cycle_detection = true if you want
        // You can either forward or override behavior here as needed.
        self.inner.next()
    }
}

impl<'a, I: Iterator<Item = Result<TraversalValue, GraphError>>> CyclePathIterator<'a, I> {
    pub fn new(
        iter: I,
        edge_label: Option<&'a str>,
        storage: Arc<HelixGraphStorage>,
        txn: &'a RoTxn<'a>,
    ) -> Self {
        CyclePathIterator {
            inner: ShortestPathIterator {
                iter,
                path_type: PathType::Cycle(true), // cycle detection using only from node
                edge_label,
                storage,
                txn,
            },
        }
    }
}

pub trait CyclePathAdapter<'a, I>: Iterator<Item = Result<TraversalValue, GraphError>>
where
    I: 'a,
{
    fn cycle_path(
        self,
        edge_label: Option<&'a str>
    ) -> RoTraversalIterator<'a, CyclePathIterator<'a, I>>;
}

impl<'a, I: Iterator<Item = Result<TraversalValue, GraphError>> + 'a> CyclePathAdapter<'a, I>
    for RoTraversalIterator<'a, I>
{
    fn cycle_path(
        self,
        edge_label: Option<&'a str>,
    ) -> RoTraversalIterator<'a, CyclePathIterator<'a, I>>
    where
        I: 'a,
    {
        // let from = from.expect("from node must be specified for cycle detection");
        let storage = Arc::clone(&self.storage);
        let txn = self.txn;

        RoTraversalIterator {
            inner: CyclePathIterator::new(
                self.inner,
                edge_label,
                storage,
                txn,
            ),
            storage: Arc::clone(&self.storage),
            txn: self.txn,
        }
    }
}