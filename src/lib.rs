use std::cell::{Ref, RefCell};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::rc::{Rc, Weak};
use std::cmp::min;

// Node structure for the tree
#[derive(Clone, Debug)]
pub struct Node<K, T>
where
    K: Eq + Hash,
{
    children: RefCell<Vec<Rc<Node<K, T>>>>,
    index: RefCell<usize>,
    parent: RefCell<Option<Weak<Node<K, T>>>>,
    value: RefCell<T>,
    key: K,
}

impl<K, T> Node<K, T>
where
    K: Eq + Hash,
{
    pub fn new(key: K, value: T) -> Rc<Self> {
        Rc::new(Node {
            children: RefCell::new(Vec::new()),
            index: RefCell::new(usize::default()),
            parent: RefCell::new(None),
            value: RefCell::new(value),
            key,
        })
    }

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

    pub fn adopt(self: &Rc<Self>, child: &Rc<Self>, index: &mut HashMap<K, Rc<Node<K, T>>>) {
        child.attach(self, index); // Pass the index map to attach
    }

    pub fn attach(self: &Rc<Self>, parent: &Rc<Self>, index: &mut HashMap<K, Rc<Node<K, T>>>) {
        self.detach(index); // Pass the index to detach
        *self.index.borrow_mut() = parent.children.borrow().len();
        *self.parent.borrow_mut() = Some(Rc::downgrade(parent));
        parent.children.borrow_mut().push(self.clone());
    }

    pub fn detach(self: &Rc<Self>, index: &mut HashMap<K, Rc<Node<K, T>>>) {
        if let Some(parent) = self.parent() {
            parent.abandon(self);
        }

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

    pub fn children(&self) -> Ref<Vec<Rc<Node<K, T>>>> {
        self.children.borrow()
    }

    pub fn is_leaf(&self) -> bool {
        self.children.borrow().is_empty()
    }

    pub fn is_root(&self) -> bool {
        self.parent.borrow().is_none()
    }

    pub fn parent(&self) -> Option<Rc<Self>> {
        self.parent.borrow().as_ref().and_then(|parent| parent.upgrade())
    }

    pub fn value(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn set_value(&self, value: T) {
        *self.value.borrow_mut() = value;
    }

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
    index: RefCell<HashMap<K, Rc<Node<K, T>>>>,  // Primary index
    secondary_index: RefCell<HashMap<String, Vec<K>>>, // Secondary index
}

impl<K, T> MultiIndexedTree<K, T>
where
    K: Eq + Hash + Ord + Clone,
    T: Clone,
{
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

    pub fn find(&self, key: &K) -> Option<Rc<Node<K, T>>> {
        self.index.borrow().get(key).cloned()
    }

    pub fn add_to_secondary_index(&self, tag: String, key: K) {
        self.secondary_index
            .borrow_mut()
            .entry(tag)
            .or_insert_with(Vec::new)
            .push(key);
    }

    pub fn find_by_secondary_index(&self, tag: &str) -> Option<Vec<Rc<Node<K, T>>>> {
        self.secondary_index
            .borrow()
            .get(tag)
            .map(|keys| keys.iter().filter_map(|k| self.find(k)).collect())
    }

    // Depth-First Iterator
    pub fn iter_depth_first(&self) -> DepthFirstIterator<K, T> {
        DepthFirstIterator {
            stack: vec![self.root.clone()],
        }
    }

    // Breadth-First Iterator
    pub fn iter_breadth_first(&self) -> BreadthFirstIterator<K, T> {
        BreadthFirstIterator {
            queue: VecDeque::from(vec![self.root.clone()]),
        }
    }

    // Shortest Path Iterator
    pub fn iter_shortest_path(&self) -> ShortestPathIterator<K, T> {
        let mut queue = VecDeque::new();
        queue.push_back((0, self.root.clone())); // Start with the root at depth 0
        ShortestPathIterator { queue }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_operations() {
        let tree = MultiIndexedTree::new("root", "root_value");

        // Insert nodes
        tree.insert(&"root", "child1", "child1_value").unwrap();
        tree.insert(&"root", "child2", "child2_value").unwrap();
        tree.insert(&"child1", "child1.1", "child1.1_value").unwrap();

        // Validate insertion
        assert_eq!(*tree.find(&"child1").unwrap().value(), "child1_value");
        assert_eq!(*tree.find(&"child2").unwrap().value(), "child2_value");
        assert_eq!(*tree.find(&"child1.1").unwrap().value(), "child1.1_value");

        // Remove a node
        tree.remove(&"child1").unwrap();
        assert!(tree.find(&"child1").is_none()); // Node should no longer exist
        assert!(tree.find(&"child1.1").is_none()); // Descendant should also be removed
    }

    #[test]
    fn test_secondary_index() {
        let tree = MultiIndexedTree::new("root", "root_value");

        // Insert nodes
        tree.insert(&"root", "child1", "child1_value").unwrap();
        tree.insert(&"root", "child2", "child2_value").unwrap();

        // Add nodes to a secondary index
        tree.add_to_secondary_index("tag1".to_string(), "child1");
        tree.add_to_secondary_index("tag1".to_string(), "child2");

        // Find nodes by secondary index
        let nodes = tree.find_by_secondary_index("tag1").unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().any(|n| *n.value() == "child1_value"));
        assert!(nodes.iter().any(|n| *n.value() == "child2_value"));
    }

    #[test]
    fn test_depth_first_iterator() {
        let tree = MultiIndexedTree::new("root", "root_value");

        // Insert nodes
        tree.insert(&"root", "child1", "child1_value").unwrap();
        tree.insert(&"root", "child2", "child2_value").unwrap();
        tree.insert(&"child1", "child1.1", "child1.1_value").unwrap();

        // Perform depth-first traversal
        let dfs: Vec<_> = tree.iter_depth_first().map(|n| n.key).collect();

        // Validate traversal order (preorder: root, left, right)
        assert_eq!(dfs, vec!["root", "child1", "child1.1", "child2"]);
    }

    #[test]
    fn test_breadth_first_iterator() {
        let tree = MultiIndexedTree::new("root", "root_value");

        // Insert nodes
        tree.insert(&"root", "child1", "child1_value").unwrap();
        tree.insert(&"root", "child2", "child2_value").unwrap();
        tree.insert(&"child1", "child1.1", "child1.1_value").unwrap();

        // Perform breadth-first traversal
        let bfs: Vec<_> = tree.iter_breadth_first().map(|n| n.key).collect();

        // Validate traversal order (level-order)
        assert_eq!(bfs, vec!["root", "child1", "child2", "child1.1"]);
    }

    #[test]
    fn test_shortest_path_iterator() {
        let tree = MultiIndexedTree::new("root", "root_value");

        // Insert nodes
        tree.insert(&"root", "child1", "child1_value").unwrap();
        tree.insert(&"root", "child2", "child2_value").unwrap();
        tree.insert(&"child1", "child1.1", "child1.1_value").unwrap();
        tree.insert(&"child2", "child2.1", "child2.1_value").unwrap();

        // Perform shortest-path traversal
        let shortest_path: Vec<_> = tree.iter_shortest_path().map(|n| n.key).collect();

        // Validate traversal order (shortest path first)
        assert_eq!(shortest_path, vec!["root", "child1", "child2", "child1.1", "child2.1"]);
    }

    #[test]
    fn test_combined_features() {
        let tree = MultiIndexedTree::new("root", "root_value");

        // Insert nodes and validate
        tree.insert(&"root", "child1", "child1_value").unwrap();
        tree.insert(&"root", "child2", "child2_value").unwrap();
        tree.insert(&"child1", "child1.1", "child1.1_value").unwrap();
        tree.insert(&"child2", "child2.1", "child2.1_value").unwrap();

        assert_eq!(*tree.find(&"child1").unwrap().value(), "child1_value");
        assert_eq!(*tree.find(&"child2").unwrap().value(), "child2_value");

        // Add nodes to secondary index and validate
        tree.add_to_secondary_index("tag1".to_string(), "child1");
        tree.add_to_secondary_index("tag1".to_string(), "child2");

        let tagged_nodes = tree.find_by_secondary_index("tag1").unwrap();
        assert_eq!(tagged_nodes.len(), 2);

        // Test iterators
        let dfs: Vec<_> = tree.iter_depth_first().map(|n| n.key).collect();
        let bfs: Vec<_> = tree.iter_breadth_first().map(|n| n.key).collect();
        let shortest: Vec<_> = tree.iter_shortest_path().map(|n| n.key).collect();

        assert_eq!(dfs, vec!["root", "child1", "child1.1", "child2", "child2.1"]);
        assert_eq!(bfs, vec!["root", "child1", "child2", "child1.1", "child2.1"]);
        assert_eq!(shortest, vec!["root", "child1", "child2", "child1.1", "child2.1"]);
    }
}
