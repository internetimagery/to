use std::cmp::Reverse;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, BinaryHeap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

/// Key, representing a type to convert from and to. eg string into an integer.
pub trait Key: Sized + Copy + Hash + Ord {}
impl<T: Sized + Copy + Hash + Ord> Key for T {}

/// Variation on a Key. Provides additional meaning/context to a key. Eg a string may also be a
/// path, a url or an email address.
pub trait Variant: Sized + Hash + Ord + Clone {}
impl<T: Sized + Hash + Ord + Clone> Variant for T {}

/// Data to be collected from the graph
pub trait Data: Sized + Hash + Ord {}
impl<T: Sized + Hash + Ord> Data for T {}

/// Collection of variations with which to pass along during the search.
type Variations<V> = BTreeSet<V>;

/// Cost of a particular action. Helps inform the search for the most optimal path to take.
type Cost = i32;

/// An edge between two nodes, representing the transformation from one type to another.
#[derive(Hash, Eq, PartialEq, Debug, Ord, PartialOrd)]
pub struct Edge<K, V, D> {
    cost: Cost,
    key_in: K,
    key_out: K,
    pub data: D,
    variations_in: Variations<V>,
    variations_out: Variations<V>,
}

type AEdge<K, V, D> = Arc<Edge<K, V, D>>;
type Edges<K, V, D> = HashMap<K, HashSet<AEdge<K, V, D>>>;
type EdgeSet<K, V, D> = BTreeSet<AEdge<K, V, D>>;

#[derive(Hash, Eq, PartialEq, Debug, Ord, PartialOrd)]
pub struct State<'a, K, V, D> {
    cost: Cost,
    var_consumed: Reverse<usize>,
    var_added: Reverse<usize>,
    variations: Variations<V>,
    edge: &'a AEdge<K, V, D>,
    parent: Option<Rc<State<'a, K, V, D>>>,
}

type RState<'a, K, V, D> = Rc<State<'a, K, V, D>>;

struct StateIter<'a, K, V, D> {
    node: Option<&'a State<'a, K, V, D>>,
}

struct Searcher<'a, K, V, D> {
    // what we have
    edges_in: &'a Edges<K, V, D>,
    edges_out: &'a Edges<K, V, D>,

    // what we want to find
    key_in: K,
    key_out: K,
    variations_in: &'a Variations<V>,
    variations_out: &'a Variations<V>,

    // our search queue
    queue_in: BinaryHeap<Reverse<RState<'a, K, V, D>>>,
    queue_out: BinaryHeap<Reverse<RState<'a, K, V, D>>>,

    // track where we have been (using u64 hash to skip tranferring ownership)
    visited_in: HashMap<&'a AEdge<K, V, D>, HashMap<u64, RState<'a, K, V, D>>>,
    visited_out: HashMap<&'a AEdge<K, V, D>, HashMap<u64, RState<'a, K, V, D>>>,

    // If we need to skip any edges in our search.
    skip_edges: &'a EdgeSet<K, V, D>,
}

// Our graph!
pub struct Graph<K, V, D> {
    edges_in: Edges<K, V, D>,
    edges_out: Edges<K, V, D>,
}

impl<'a, K: Key, V: Variant, D: Data> State<'a, K, V, D> {
    fn new(
        mut var_consumed: usize,
        mut var_added: usize,
        edge: &'a AEdge<K, V, D>,
        parent: Option<RState<'a, K, V, D>>,
        variations: Variations<V>,
    ) -> Self {
        let cost;
        match &parent {
            Some(p) => {
                cost = p.cost + edge.cost;
                let Reverse(parent_var_consumed) = p.var_consumed;
                var_consumed += parent_var_consumed;
                let Reverse(parent_var_added) = p.var_added;
                var_added += parent_var_added;
            }
            None => {
                cost = edge.cost;
            }
        };

        Self {
            cost,
            var_consumed: Reverse(var_consumed),
            var_added: Reverse(var_added),
            edge,
            parent,
            variations,
        }
    }
    fn iter(&self) -> StateIter<K, V, D> {
        StateIter { node: Some(self) }
    }
}

impl<'a, K: Key, V: Variant, D: Data> Iterator for StateIter<'a, K, V, D> {
    type Item = &'a State<'a, K, V, D>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            self.node = match &node.parent {
                Some(parent) => Some(parent),
                None => None,
            };
            return Some(node);
        }
        None
    }
}

impl<'a, K: Key, V: Variant, D: Data> Searcher<'a, K, V, D> {
    fn new(
        key_in: K,
        variations_in: &'a Variations<V>,
        key_out: K,
        variations_out: &'a Variations<V>,
        edges_in: &'a Edges<K, V, D>,
        edges_out: &'a Edges<K, V, D>,
        skip_edges: &'a EdgeSet<K, V, D>,
    ) -> Self {
        Searcher {
            edges_in,
            edges_out,
            key_in,
            key_out,
            variations_in,
            variations_out,
            queue_in: BinaryHeap::new(),
            queue_out: BinaryHeap::new(),
            visited_in: HashMap::new(),
            visited_out: HashMap::new(),
            skip_edges,
        }
    }

    /// Look for the cheapest path between converters (edges)
    /// A chain of types must match. eg A>B  B>C C>D
    /// Variations are like dependencies on input. They are required
    /// to satisfy that edges traversal. If an edge does not satisfy
    /// the right variations, it can be visited again later when it
    /// the current state has a different set of variations.
    fn search(&mut self) -> Option<Vec<AEdge<K, V, D>>> {
        self.set_queue_in();
        self.set_queue_out();

        // Loop till we run out of options.
        // Search forward and back at the same time.
        // Favour the direction with the least number of options.
        loop {
            if !self.queue_in.is_empty()
                && (self.queue_in.len() < self.queue_out.len() || self.queue_out.is_empty())
            {
                if let Some(result) = self.search_forward() {
                    return Some(result);
                }
            } else if !self.queue_out.is_empty() {
                if let Some(result) = self.search_backward() {
                    return Some(result);
                }
            } else {
                break;
            }
        }
        None
    }

    fn search_forward(&mut self) -> Option<Vec<AEdge<K, V, D>>> {
        // next state
        let state = match self.queue_in.pop() {
            Some(Reverse(s)) => s,
            _ => return None,
        };

        if self.skip_edges.contains(state.edge) {
            return None;
        }

        // Check if we have reached our goal and variations are all met
        if state.edge.key_out == self.key_out && state.variations.is_superset(self.variations_out) {
            return Some(
                state
                    .iter()
                    .map(|s| Arc::clone(&s.edge))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect(),
            );
        }

        // Check if our path intersects the forward search
        if let Some(opposite_states) = self.visited_out.get(&state.edge) {
            for opposite_state in opposite_states.values() {
                // Dependency check
                if !opposite_state.variations.is_subset(match &state.parent {
                    Some(parent) => &parent.variations,
                    None => self.variations_in,
                }) {
                    continue;
                }
                return Some(
                    state
                        .iter()
                        .skip(1)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .chain(opposite_state.iter())
                        .map(|s| Arc::clone(&s.edge))
                        .collect(),
                );
            }
        }

        // Mark where we have been
        let edge_entry = self.visited_in.entry(state.edge).or_insert(HashMap::new());
        edge_entry.insert(
            match &state.parent {
                Some(parent) => hash(&parent.variations),
                None => hash(&self.variations_in),
            },
            Rc::clone(&state),
        );
        self.add_queue_in(state);
        None
    }

    fn search_backward(&mut self) -> Option<Vec<AEdge<K, V, D>>> {
        let state = match self.queue_out.pop() {
            Some(Reverse(s)) => s,
            _ => return None,
        };

        if self.skip_edges.contains(state.edge) {
            return None;
        }

        // Check if we have reached our goal and variations dependencies are met
        if state.edge.key_in == self.key_in && state.variations.is_subset(self.variations_in) {
            return Some(state.iter().map(|s| Arc::clone(&s.edge)).collect());
        }

        // Check if our path intersects the forward search
        if let Some(opposite_states) = self.visited_in.get(&state.edge) {
            for opposite_state in opposite_states.values() {
                // Dependency check
                if !state.variations.is_subset(match &opposite_state.parent {
                    Some(parent) => &parent.variations,
                    None => self.variations_in,
                }) {
                    continue;
                }

                return Some(
                    opposite_state
                        .iter()
                        .skip(1)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .chain(state.iter())
                        .map(|s| Arc::clone(&s.edge))
                        .collect(),
                );
            }
        }

        // Mark where we have been
        let edge_entry = self.visited_out.entry(state.edge).or_insert(HashMap::new());
        edge_entry.insert(
            match &state.parent {
                Some(parent) => hash(&parent.variations),
                None => hash(&self.variations_out),
            },
            Rc::clone(&state),
        );

        self.add_queue_out(state);
        None
    }

    fn set_queue_in(&mut self) {
        if let Some(edges) = self.edges_in.get(&self.key_in) {
            for edge in edges {
                // Variation requirement check
                if !edge.variations_in.is_subset(self.variations_in) {
                    continue;
                }
                // This is a subset so we know it's <= to total
                // Prioritize nodes that match more of our variations
                let var_consumed = edge.variations_in.len();
                self.queue_in.push(Reverse(Rc::new(State::new(
                    var_consumed,
                    edge.variations_out.len(),
                    &edge,
                    None,
                    &(self.variations_in - &edge.variations_in) | &edge.variations_out,
                ))))
            }
        }
    }

    fn set_queue_out(&mut self) {
        if let Some(edges) = self.edges_out.get(&self.key_out) {
            for edge in edges {
                let var_consumed = edge
                    .variations_out
                    .intersection(&self.variations_out)
                    .count();
                self.queue_out.push(Reverse(Rc::new(State::new(
                    var_consumed,
                    edge.variations_in.len(),
                    &edge,
                    None,
                    &(self.variations_out - &edge.variations_out) | &edge.variations_in,
                ))))
            }
        }
    }

    fn add_queue_in(&mut self, state: RState<'a, K, V, D>) {
        if let Some(edges) = self.edges_in.get(&state.edge.key_out) {
            for edge in edges {
                if self
                    .visited_in
                    .get(&edge)
                    .map_or(false, |v| v.contains_key(&hash(&state.variations)))
                {
                    continue;
                }
                // Variation dependency check
                // Any node visited walking forward needs all its
                // variations to be provided. By the current node and
                // all nodes that came before (unless they already
                // consumed some)
                if !edge.variations_in.is_subset(&state.variations) {
                    continue;
                }

                // Adjust our variations.
                // Penalize nodes that take less variations.
                // So we prioritize nodes that are more specific.
                let var_consumed = state.variations.intersection(&edge.variations_in).count();
                self.queue_in.push(Reverse(Rc::new(State::new(
                    var_consumed,
                    edge.variations_out.len(),
                    &edge,
                    Some(Rc::clone(&state)),
                    &(&state.variations - &edge.variations_in) | &edge.variations_out,
                ))));
            }
        }
    }

    fn add_queue_out(&mut self, state: RState<'a, K, V, D>) {
        // Search further into the graph!
        if let Some(edges) = self.edges_out.get(&state.edge.key_in) {
            for edge in edges {
                if self
                    .visited_out
                    .get(&edge)
                    .map_or(false, |v| v.contains_key(&hash(&state.variations)))
                {
                    continue;
                }
                // No dependency check going in reverse. As dependencies
                // could be satisfied further down the chain.
                // Prioritize nodes that reduce our variation count more
                let var_consumed = state.variations.intersection(&edge.variations_out).count();
                self.queue_out.push(Reverse(Rc::new(State::new(
                    var_consumed,
                    edge.variations_in.len(),
                    &edge,
                    Some(Rc::clone(&state)),
                    &(&state.variations - &edge.variations_out) | &edge.variations_in,
                ))));
            }
        }
    }
}

impl<K: Key, V: Variant, D: Data> Graph<K, V, D> {
    // Create a new graph
    pub fn new() -> Self {
        Graph {
            edges_in: HashMap::new(),
            edges_out: HashMap::new(),
        }
    }

    // Add new edges to the graph
    pub fn add_edge(
        &mut self,
        cost: Cost,
        key_in: K,
        variations_in: Variations<V>,
        key_out: K,
        variations_out: Variations<V>,
        data: D,
    ) {
        let edge_arc = Arc::new(Edge {
            cost,
            key_in,
            key_out,
            data,
            variations_in,
            variations_out,
        });
        let edges_in = self.edges_in.entry(key_in).or_insert(HashSet::new());
        let edges_out = self.edges_out.entry(key_out).or_insert(HashSet::new());
        edges_in.insert(Arc::clone(&edge_arc));
        edges_out.insert(edge_arc);
    }

    // Search the graph to find what we want to find
    pub fn search(
        &self,
        key_in: K,
        variations_in: &Variations<V>,
        key_out: K,
        variations_out: &Variations<V>,
        skip_edges: &EdgeSet<K, V, D>,
    ) -> Option<Vec<AEdge<K, V, D>>> {
        let mut searcher = Searcher::new(
            key_in,
            variations_in,
            key_out,
            variations_out,
            &self.edges_in,
            &self.edges_out,
            skip_edges,
        );
        searcher.search()
    }
}

fn hash<H>(hashable: H) -> u64
where
    H: Hash,
{
    let mut hasher = DefaultHasher::new();
    hashable.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! _set {
        ( $($val:expr),* ) => {
            {
                let mut _set = BTreeSet::new();
                $(
                    _set.insert($val);
                )*
                _set
            }
        }
    }

    macro_rules! _graph {
        ( $(($cost:expr, $in:expr, {$($in_var:tt)*}, $out:expr, {$($out_var:tt)*}, $func:expr)),*) => {
            {
                let mut graph: Graph<u64, u64, u64> = Graph::new();
                $(
                    graph.add_edge($cost, $in, _set!($($in_var)*), $out, _set!($($out_var)*), $func);
                )*
                graph
            }
        }
    }

    macro_rules! _setup {
        ( $searcher:ident, [$in:expr, {$($in_var:tt)*}, $out:expr, {$($out_var:tt)*}], [$($graph:tt)*], $body:block ) => {
            {
            let graph = _graph!($($graph)*);
            let variations_in = _set!($($in_var)*);
            let variations_out = _set!($($out_var)*);
            let skip_null = BTreeSet::new();
            let mut $searcher = Searcher::new(
                $in,
                &variations_in,
                $out,
                &variations_out,
                &graph.edges_in,
                &graph.edges_out,
                &skip_null,
            );
            $searcher.set_queue_in();
            $searcher.set_queue_out();
            $body
            }
        }
    }

    #[test]
    fn test_forward_no_path() {
        let result = _setup!(s, [1, {}, 2, {}], [(1, 2, {}, 3, {}, 1)], {
            s.search_forward()
        })
        .is_some();
        assert_eq!(result, false);
    }

    #[test]
    fn test_backward_no_path() {
        let result = _setup!(s, [1, {}, 2, {}], [(1, 2, {}, 3, {}, 1)], {
            s.search_backward()
        })
        .is_some();
        assert_eq!(result, false);
    }

    #[test]
    fn test_forward_no_path_variations() {
        let result = _setup!(s, [1, {}, 2, {}], [(1, 1, { 1 }, 2, {}, 1)], {
            s.search_forward()
        })
        .is_some();
        assert_eq!(result, false);
    }

    #[test]
    fn test_backward_no_path_variations() {
        let result = _setup!(s, [1, {}, 2, {}], [(1, 1, { 1 }, 2, {}, 1)], {
            s.search_backward()
        })
        .is_some();
        assert_eq!(result, false);
    }

    #[test]
    fn test_forward_one_step() {
        let result = _setup!(s, [1, {}, 2, {}], [(1, 1, {}, 2, {}, 1)], {
            s.search_forward()
        })
        .unwrap();
        assert_eq!(result[0].data, 1);
    }

    #[test]
    fn test_forward_one_step_variations_in() {
        let result = _setup!(s, [1, { 1 }, 2, {}], [(1, 1, { 1 }, 2, {}, 1)], {
            s.search_forward()
        })
        .unwrap();
        assert_eq!(result[0].data, 1);
    }

    #[test]
    fn test_forward_one_step_variations_out() {
        let result = _setup!(
            s,
            [1, {}, 2, { 1 }],
            [(5, 1, {}, 2, { 1 }, 1), (1, 1, {}, 2, {}, 2)],
            {
                s.search_forward();
                s.search_forward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
    }

    #[test]
    fn test_backward_one_step() {
        let result = _setup!(s, [1, {}, 2, {}], [(1, 1, {}, 2, {}, 1)], {
            s.search_backward()
        })
        .unwrap();
        assert_eq!(result[0].data, 1);
    }

    #[test]
    fn test_backward_one_step_variations_in() {
        let result = _setup!(s, [1, { 1 }, 2, {}], [(1, 1, { 1 }, 2, {}, 1)], {
            s.search_backward()
        })
        .unwrap();
        assert_eq!(result[0].data, 1);
    }

    #[test]
    fn test_backward_one_step_variations_out() {
        let result = _setup!(
            s,
            [1, {}, 2, { 1 }],
            [(5, 1, {}, 2, { 1 }, 1), (1, 1, {}, 2, {}, 2)],
            {
                s.search_backward();
                s.search_backward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
    }

    #[test]
    fn test_forward_two_step() {
        let result = _setup!(
            s,
            [1, {}, 3, {}],
            [(1, 1, {}, 2, {}, 1), (1, 2, {}, 3, {}, 2)],
            {
                s.search_forward();
                s.search_forward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 2);
    }

    #[test]
    fn test_backward_two_step() {
        let result = _setup!(
            s,
            [1, {}, 3, {}],
            [(1, 1, {}, 2, {}, 1), (1, 2, {}, 3, {}, 2)],
            {
                s.search_backward();
                s.search_backward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 2);
    }

    #[test]
    fn test_forward_cheapest() {
        let result = _setup!(
            s,
            [1, {}, 3, {}],
            [
                (1, 1, {}, 2, {}, 1),
                (2, 2, {}, 3, {}, 2),
                (1, 2, {}, 3, {}, 3)
            ],
            {
                s.search_forward();
                s.search_forward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 3);
    }

    #[test]
    fn test_forward_cheapest_variations_in() {
        let result = _setup!(
            s,
            [1, { 1 }, 3, {}],
            [
                (1, 1, {}, 2, {}, 1),
                (1, 2, { 1 }, 3, {}, 2),
                (1, 2, {}, 3, {}, 3)
            ],
            {
                s.search_forward();
                s.search_forward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 2);
    }

    #[test]
    fn test_backward_cheapest() {
        let result = _setup!(
            s,
            [1, {}, 3, {}],
            [
                (1, 1, {}, 2, {}, 1),
                (2, 2, {}, 3, {}, 2),
                (1, 2, {}, 3, {}, 3)
            ],
            {
                s.search_backward();
                s.search_backward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 3);
    }

    #[test]
    fn test_backward_cheapest_variations_in() {
        let result = _setup!(
            s,
            [1, { 1 }, 3, {}],
            [
                (1, 1, {}, 2, {}, 1),
                (1, 2, { 1 }, 3, {}, 2),
                (1, 2, {}, 3, {}, 3)
            ],
            {
                s.search_backward();
                s.search_backward();
                s.search_backward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 2);
    }

    #[test]
    fn test_backward_cheapest_variations_out() {
        let result = _setup!(
            s,
            [1, {}, 4, { 1 }],
            [
                (1, 1, {}, 2, {}, 1),
                (1, 2, {}, 3, { 1 }, 2),
                (1, 2, {}, 3, {}, 3),
                (1, 3, {}, 4, {}, 4)
            ],
            {
                s.search_backward();
                s.search_backward();
                s.search_backward();
                s.search_backward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 2);
        assert_eq!(result[2].data, 4);
    }

    #[test]
    fn test_forward_intersect() {
        let result = _setup!(
            s,
            [1, {}, 4, {}],
            [
                (1, 1, {}, 2, {}, 1),
                (1, 2, {}, 3, {}, 2),
                (1, 3, {}, 4, {}, 3)
            ],
            {
                s.search_forward();
                s.search_forward();
                s.search_backward();
                s.search_forward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 2);
        assert_eq!(result[2].data, 3);
    }

    #[test]
    fn test_backward_intersect() {
        let result = _setup!(
            s,
            [1, {}, 4, {}],
            [
                (1, 1, {}, 2, {}, 1),
                (1, 2, {}, 3, {}, 2),
                (1, 3, {}, 4, {}, 3)
            ],
            {
                s.search_backward();
                s.search_backward();
                s.search_forward();
                s.search_backward()
            }
        )
        .unwrap();
        assert_eq!(result[0].data, 1);
        assert_eq!(result[1].data, 2);
        assert_eq!(result[2].data, 3);
    }

    #[test]
    fn test_search_variation() {
        let result = _setup!(
            s,
            [1, { 1 }, 3, {}],
            [
                (1, 1, {}, 2, {}, 1),
                (1, 1, { 1 }, 4, {}, 2),
                (1, 2, {}, 3, {}, 3),
                (1, 4, {}, 3, {}, 4)
            ],
            { s.search() }
        )
        .unwrap();
        assert_eq!(result[0].data, 2);
        assert_eq!(result[1].data, 4);
    }
}
