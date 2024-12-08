use std::cell::{Ref, RefCell};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::rc::{Rc, Weak};
use std::cmp::min;


/// TODO:
/// Error Handling
/// Edge Weights (just assuming theyre unweghted for now) 
/// parallelism (this way we can use iterators like rayon for traversal), 
/// need to add better documentation,
/// serialization for saving/loading the tree (serde), 
/// Display trait so that it can visualize the tree struct?? 
/// 
/// Optimization: consider if Rc and RefCell for all cases is really neccesary..  
/// or maybe a combination of mutable refs can optimize certain operations.

// Node structure for the tree
#[derive(Clone, Debug)]
pub struct Node<K, T>
where
    K: Eq + Hash,
{
    // A list of child nodes
    children: RefCell<Vec<Rc<Node<K, T>>>>,
    // Index of the node in its parent's children vector
    index: RefCell<usize>,
    // Parent of the current node
    parent: RefCell<Option<Weak<Node<K, T>>>>,
    // The value stored in this node
    value: RefCell<T>,
    // Unique key to identify the node
    key: K,
}

impl<K, T> Node<K, T>
where
    K: Eq + Hash,
{
    // Create a new node with a given key and value
    pub fn new(key: K, value: T) -> Rc<Self> {
        Rc::new(Node {
            children: RefCell::new(Vec::new()),
            index: RefCell::new(usize::default()),
            parent: RefCell::new(None),
            value: RefCell::new(value),
            key,
        })
    }

    // Detach a specific child from the node
    pub fn abandon(&self, child: &Rc<Self>) {
        let index = *child.index.borrow();
        *child.parent.borrow_mut() = None;
        self.children.borrow_mut().swap_remove(index);

        let count = self.children.borrow().len();

        if count != 0 {
            let index = min(index, count - 1);
            *self.children.borrow_mut()[index].index.borrow_mut() = index;
        }
    }

    // Adopt a new child node under the current node
    pub fn adopt(self: &Rc<Self>, child: &Rc<Self>, index: &mut HashMap<K, Rc<Node<K, T>>>) {
        child.attach(self, index); // Pass the index map to attach
    }

    // Attach this node to a new parent
    pub fn attach(self: &Rc<Self>, parent: &Rc<Self>, index: &mut HashMap<K, Rc<Node<K, T>>>) {
        self.detach(index); // Pass the index to detach
        *self.index.borrow_mut() = parent.children.borrow().len();
        *self.parent.borrow_mut() = Some(Rc::downgrade(parent));
        parent.children.borrow_mut().push(self.clone());
    }

    // Detach this node from its current parent
    pub fn detach(self: &Rc<Self>, index: &mut HashMap<K, Rc<Node<K, T>>>) {
        if let Some(parent) = self.parent() {
            parent.abandon(self);
        }

        // Recursively remove all descendants of this node from the index
        fn remove_descendants<K, T>(node: &Rc<Node<K, T>>, index: &mut HashMap<K, Rc<Node<K, T>>>)
        where
            K: Eq + Hash,
        {
            for child in node.children.borrow().iter() {
                remove_descendants(child, index);
            }
            index.remove(&node.key);
        }

        remove_descendants(self, index);
    }

    // Get a reference to the children of this node
    pub fn children(&self) -> Ref<Vec<Rc<Node<K, T>>>> {
        self.children.borrow()
    }

    // Check if the node is a leaf (has no children)
    pub fn is_leaf(&self) -> bool {
        self.children.borrow().is_empty()
    }

    // Check if the node is the root (has no parent)
    pub fn is_root(&self) -> bool {
        self.parent.borrow().is_none()
    }

    // Get a reference to the parent node, if it exists
    pub fn parent(&self) -> Option<Rc<Self>> {
        self.parent.borrow().as_ref().and_then(|parent| parent.upgrade())
    }

    // Get the value stored in this node
    pub fn value(&self) -> Ref<T> {
        self.value.borrow()
    }

    // Set the value stored in this node
    pub fn set_value(&self, value: T) {
        *self.value.borrow_mut() = value;
    }

    // Get the unique key of this node
    pub fn key(&self) -> &K {
        &self.key
    }
}

// Multi-Indexed Tree structure
#[derive(Debug)]
pub struct MultiIndexedTree<K, T>
where
    K: Eq + Hash + Ord,
{
    root: Rc<Node<K, T>>,
    index: RefCell<HashMap<K, Rc<Node<K, T>>>>,  // Primary index for quick lookup by key
    secondary_index: RefCell<HashMap<String, Vec<K>>>, // Secondary index for grouping by tags
}

impl<K, T> MultiIndexedTree<K, T>
where
    K: Eq + Hash + Ord + Clone,
    T: Clone,
{
    // Create a new tree with a root node
    pub fn new(root_key: K, root_value: T) -> Self {
        let root = Node::new(root_key.clone(), root_value);
        let mut index = HashMap::new();
        index.insert(root_key, root.clone());

        Self {
            root,
            index: RefCell::new(index),
            secondary_index: RefCell::new(HashMap::new()),
        }
    }

    // Insert a new node under the given parent key
    pub fn insert(&self, parent_key: &K, key: K, value: T) -> Result<(), String> {
        let parent = self.index.borrow().get(parent_key).cloned();

        match parent {
            Some(parent_node) => {
                let new_node = Node::new(key.clone(), value);
                parent_node.adopt(&new_node, &mut self.index.borrow_mut()); // Pass the index map

                self.index.borrow_mut().insert(key, new_node);
                Ok(())
            }
            None => Err("Parent key not found".to_string()),
        }
    }

    // Remove a node by key
    pub fn remove(&self, key: &K) -> Result<(), String> {
        let node = self.index.borrow().get(key).cloned();

        match node {
            Some(node) => {
                node.detach(&mut self.index.borrow_mut());
                Ok(())
            }
            None => Err("Key not found".to_string()),
        }
    }

    // Find a node by its key
    pub fn find(&self, key: &K) -> Option<Rc<Node<K, T>>> {
        self.index.borrow().get(key).cloned()
    }

    // Add a key to the secondary index under a specific tag
    pub fn add_to_secondary_index(&self, tag: String, key: K) {
        self.secondary_index
            .borrow_mut()
            .entry(tag)
            .or_insert_with(Vec::new)
            .push(key);
    }

    // Find all nodes associated with a specific tag in the secondary index
    pub fn find_by_secondary_index(&self, tag: &str) -> Option<Vec<Rc<Node<K, T>>>> {
        self.secondary_index
            .borrow()
            .get(tag)
            .map(|keys| keys.iter().filter_map(|k| self.find(k)).collect())
    }

    // Create a depth-first iterator for the tree
    pub fn iter_depth_first(&self) -> DepthFirstIterator<K, T> {
        DepthFirstIterator {
            stack: vec![self.root.clone()],
        }
    }

    // Create a breadth-first iterator for the tree
    pub fn iter_breadth_first(&self) -> BreadthFirstIterator<K, T> {
        BreadthFirstIterator {
            queue: VecDeque::from(vec![self.root.clone()]),
        }
    }

    // Create a shortest path iterator
    pub fn iter_shortest_path(&self) -> ShortestPathIterator<K, T> {
        let mut queue = VecDeque::new();
        queue.push_back((0, self.root.clone())); // Start with the root at depth 0
        ShortestPathIterator { queue }
    }

    // Implement Dijkstra's algorithm to find all shortest paths from start to end
    pub fn dijkstra_shortest_paths(
        &self,
        start_key: &K,
        end_key: &K,
    ) -> Option<HashMap<usize, Vec<K>>> {
        let mut distances: HashMap<K, usize> = HashMap::new();
        let mut predecessors: HashMap<K, Vec<K>> = HashMap::new();
        let mut queue = VecDeque::new();

        // Initialize distances
        for key in self.index.borrow().keys() {
            distances.insert(key.clone(), usize::MAX);
        }

        // Set the start node's distance to 0
        distances.insert(start_key.clone(), 0);
        queue.push_back(start_key.clone());

        // Perform the BFS-style traversal
        while let Some(current_key) = queue.pop_front() {
            let current_distance = *distances.get(&current_key).unwrap();
            if let Some(node) = self.index.borrow().get(&current_key) {
                for child in node.children.borrow().iter() {
                    let child_key = child.key.clone();
                    let new_distance = current_distance + 1; // Assuming unweighted edges (weight = 1)

                    if new_distance < *distances.get(&child_key).unwrap() {
                        distances.insert(child_key.clone(), new_distance);
                        predecessors.insert(child_key.clone(), vec![current_key.clone()]);
                        queue.push_back(child_key.clone());
                    } else if new_distance == *distances.get(&child_key).unwrap() {
                        // Add an additional predecessor for ties in shortest paths
                        predecessors
                            .entry(child_key.clone())
                            .or_default()
                            .push(current_key.clone());
                    }
                }
            }
        }

        // Trace back from the end node to gather paths
        fn trace_paths<K: Clone + Eq + Hash>(
            end_key: &K,
            predecessors: &HashMap<K, Vec<K>>,
            current_path: &mut Vec<K>,
            all_paths: &mut HashMap<usize, Vec<K>>,
        ) {
            current_path.push(end_key.clone());
            if let Some(preds) = predecessors.get(end_key) {
                for pred in preds {
                    trace_paths(pred, predecessors, current_path, all_paths);
                }
            } else {
                // Base case: no predecessor, add reversed path
                let mut path = current_path.clone();
                path.reverse();
                all_paths.insert(path.len(), path);
            }
            current_path.pop();
        }

        if distances[end_key] == usize::MAX {
            return None; // No path exists
        }

        let mut all_paths = HashMap::new();
        let mut current_path = Vec::new();
        trace_paths(end_key, &predecessors, &mut current_path, &mut all_paths);

        Some(all_paths)
    }
}

// Depth-First Iterator
pub struct DepthFirstIterator<K, T>
where
    K: Eq + Hash,
{
    stack: Vec<Rc<Node<K, T>>>,
}

impl<K, T> Iterator for DepthFirstIterator<K, T>
where
    K: Eq + Hash,
{
    type Item = Rc<Node<K, T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop() {
            for child in node.children.borrow().iter().rev() {
                self.stack.push(child.clone());
            }
            Some(node)
        } else {
            None
        }
    }
}

// Breadth-First Iterator
pub struct BreadthFirstIterator<K, T>
where
    K: Eq + Hash,
{
    queue: VecDeque<Rc<Node<K, T>>>,
}

impl<K, T> Iterator for BreadthFirstIterator<K, T>
where
    K: Eq + Hash,
{
    type Item = Rc<Node<K, T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.queue.pop_front() {
            for child in node.children.borrow().iter() {
                self.queue.push_back(child.clone());
            }
            Some(node)
        } else {
            None
        }
    }
}

// Shortest Path Iterator
pub struct ShortestPathIterator<K, T>
where
    K: Eq + Hash + Ord,
{
    queue: VecDeque<(usize, Rc<Node<K, T>>)>, // Queue with depth tracking
}

impl<K, T> Iterator for ShortestPathIterator<K, T>
where
    K: Eq + Hash + Ord,
{
    type Item = Rc<Node<K, T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((depth, node)) = self.queue.pop_front() {
            for child in node.children.borrow().iter() {
                self.queue.push_back((depth + 1, child.clone())); // Add children with incremented depth
            }
            Some(node)
        } else {
            None
        }
    }
}
