use crate::{
    helix_engine::{
        traversal_core::{traversal_value::TraversalValue, traversal_iter::RoTraversalIterator},
        storage_core::{HelixGraphStorage, storage_methods::StorageMethods},
        types::GraphError,
    },
    utils::{label_hash::hash_label},
};
use heed3::RoTxn;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub enum PathType {
    From(u128),
    To(u128),
}

pub struct CyclePathIterator<'a, I> {
    iter: I,
    path_type: PathType,
    is_cycle_detection: Option<&'a bool>,
    edge_label: Option<&'a str>,
    storage: Arc<HelixGraphStorage>,
    txn: &'a RoTxn<'a>,
}

impl<'a, I: Iterator<Item = Result<TraversalValue, GraphError>>> Iterator
    for CyclePathIterator<'a, I>
{
    type Item = Result<TraversalValue, GraphError>;

    /// Returns the next outgoing node by decoding the edge id and then getting the edge and node
    fn next(&mut self) -> Option<Self::Item> {
        println!("hallo");
        match self.iter.next() {
            Some(Ok(TraversalValue::Node(node))) => {
                let (from, to) = match self.path_type {
                    PathType::From(from) => (from, node.id),
                    PathType::To(to) => (node.id, to),
                };
                let is_cycle_detection: bool = self.is_cycle_detection.map_or(false, |val| *val);

                let mut queue = VecDeque::with_capacity(32);
                let mut visited = HashSet::with_capacity(64);
                let mut parent: HashMap<u128, (u128, u128)> = HashMap::with_capacity(32);
                queue.push_back(from);
                visited.insert(from);

                let reconstruct_path = |parent: &HashMap<u128, (u128, u128)>,
                                        start_id: &u128,
                                        end_id: &u128, 
                                        is_cycle_detection: &bool
                                        |
                 -> Result<TraversalValue, GraphError> {
                    println!("parent: {:?}", parent);
                    let mut nodes = Vec::with_capacity(parent.len());
                    let mut edges = Vec::with_capacity(parent.len() - 1);

                    let mut current = end_id;

                    let mut is_visted_end_node: bool = false;
                    while (current != start_id) || (*is_cycle_detection && !is_visted_end_node) {
                        is_visted_end_node = true;
                        nodes.push(self.storage.get_node(self.txn, current)?);

                        let (prev_node, edge) = &parent[current];
                        edges.push(self.storage.get_edge(self.txn, edge)?);
                        current = prev_node;
                    }

                    nodes.push(self.storage.get_node(self.txn, start_id)?);

                    nodes.reverse();
                    edges.reverse();

                    Ok(TraversalValue::Path((nodes, edges)))
                };

                while let Some(current_id) = queue.pop_front() {
                    println!("current_id: {:?}", current_id);
                    let out_prefix = self.edge_label.map_or_else(
                        || current_id.to_be_bytes().to_vec(),
                        |label| {
                            HelixGraphStorage::out_edge_key(&current_id, &hash_label(label, None))
                                .to_vec()
                        },
                    );

                    let iter = self
                        .storage
                        .out_edges_db
                        .prefix_iter(self.txn, &out_prefix)
                        .unwrap();

                    for result in iter {
                        let (_, value) = result.unwrap(); // TODO: handle error
                        let (edge_id, to_node) =
                            HelixGraphStorage::unpack_adj_edge_data(value).unwrap(); // TODO: handle error

                        if (is_cycle_detection && to_node == to ) || !visited.contains(&to_node) {
                            visited.insert(to_node);
                            parent.insert(to_node, (current_id, edge_id));

                            if to_node == to {
                                return Some(reconstruct_path(&parent, &from, &to, &is_cycle_detection));
                            }

                            queue.push_back(to_node);
                        }
                    }
                }
                Some(Err(GraphError::ShortestPathNotFound))
            }
            Some(other) => Some(other),
            None => None,
        }
    }
}

pub trait ShortestPathAdapter<'a, I>: Iterator<Item = Result<TraversalValue, GraphError>> {
    /// ShortestPath finds the shortest path between two nodes
    ///
    /// # Arguments
    ///
    /// * `edge_label` - The label of the edge to use
    /// * `from` - The starting node
    /// * `to` - The ending node
    ///
    /// # Example
    ///
    /// ```rust
    /// let node1 = Node { id: 1, label: "Person".to_string(), properties: None };
    /// let node2 = Node { id: 2, label: "Person".to_string(), properties: None };
    /// let traversal = G::new(storage, &txn).shortest_path(Some("knows"), Some(&node1.id), Some(&node2.id));
    /// ```
    fn shortest_path(
        self,
        edge_label: Option<&'a str>,
        from: Option<&'a u128>,
        to: Option<&'a u128>,
        is_cycle_detection: Option<&'a bool>,
    ) -> RoTraversalIterator<'a, ShortestPathIterator<'a, I>>
    where
        I: 'a;
}

impl<'a, I: Iterator<Item = Result<TraversalValue, GraphError>> + 'a> ShortestPathAdapter<'a, I>
    for RoTraversalIterator<'a, I>
{
    #[inline]
    fn shortest_path(
        self,
        edge_label: Option<&'a str>,
        from: Option<&'a u128>,
        to: Option<&'a u128>,
        is_cycle_detection: Option<&'a bool>
    ) -> RoTraversalIterator<'a, ShortestPathIterator<'a, I>>
    where
        I: 'a,
    {
        let storage = Arc::clone(&self.storage);
        let txn = self.txn;

        let is_cycle_detection: Option<&'a bool> = is_cycle_detection.or(Some(&false));

        println!("input from: {:?}", from);
        println!("input to: {:?}", to);
        RoTraversalIterator {
            inner: ShortestPathIterator {
                iter: self.inner,
                path_type: match (from, to) {
                    (Some(from), None) => PathType::From(*from),
                    (None, Some(to)) => PathType::To(*to),
                    _ => panic!("Invalid shortest path"),
                },
                is_cycle_detection,
                edge_label,
                storage,
                txn,
            },
            storage: Arc::clone(&self.storage),
            txn: self.txn,
        }
    }
}
