# Multi-Indexed Tree in Rust

A generic, flexible, and lightweight **multi-indexed tree data structure** written in Rust.
This library provides hierarchical node storage with support for **fast key lookups**, **tag-based indexing**, and multiple traversal strategies.

## Features

* **Generic Node Structure** — Supports any key (`K`) that implements `Eq + Hash + Ord` and any value (`T`).
* **Parent & Child Relationships** — Each node tracks its parent and children.
* **Primary Indexing** — Constant-time lookups using a `HashMap` keyed by node IDs.
* **Secondary Indexing** — Group nodes under arbitrary tags for quick retrieval.
* **Tree Traversal**:

  * Depth-first iterator
  * Breadth-first iterator
  * Shortest-path iterator
* **Dijkstra-based Shortest Path** — Finds all shortest paths between two nodes (assuming unweighted edges).
* **Safe Memory Management** — Built with `Rc` + `RefCell` for shared ownership and interior mutability.

## Usage

```rust
mod tree;
use tree::MultiIndexedTree;

fn main() {
    // Create a tree with a root node
    let tree = MultiIndexedTree::new("root", "Root Node");

    // Insert child nodes
    tree.insert(&"root", "child1", "First Child").unwrap();
    tree.insert(&"root", "child2", "Second Child").unwrap();

    // Add tags to nodes
    tree.add_to_secondary_index("important".to_string(), "child1".to_string());

    // Lookup nodes by key
    if let Some(node) = tree.find(&"child1") {
        println!("Found: {}", node.value().clone());
    }

    // Depth-first traversal
    for node in tree.iter_depth_first() {
        println!("Node: {}", node.key());
    }
}
```

## Roadmap / TODOs

* [ ] Better error handling
* [ ] Edge weights for weighted shortest paths
* [ ] Parallel traversal with Rayon
* [ ] Serialization & deserialization via Serde
* [ ] Implement the `Display` trait for visualization
* [ ] Optimize `Rc<RefCell<...>>` usage where possible

## License

MIT License — Free to use, modify, and distribute.
