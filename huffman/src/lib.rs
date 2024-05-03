use bitvec::order::Lsb0;
use bitvec::slice::BitValIter;
use bytes::{BufMut, BytesMut};

// if this is actually index into the arena, can't be outsid of MAX_NODES
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct NodeIndex(usize);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct NodeWeight(u64);

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct Symbol(u8);

#[derive(/*Copy, Clone,*/ Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Node {
    NotYetTransmitted {
        parent: Option<NodeIndex>,
    },
    Leaf {
        parent: NodeIndex,

        weight: NodeWeight,

        symbol: Symbol,
    },
    Internal {
        parent: Option<NodeIndex>,

        left: NodeIndex,
        right: NodeIndex,

        weight: NodeWeight,
    },
}

impl Node {
    fn parent_index(&self) -> Option<NodeIndex> {
        match *self {
            Node::NotYetTransmitted { parent } => parent,
            Node::Leaf { parent, .. } => Some(parent),
            Node::Internal { parent, .. } => parent,
        }
    }

    fn weight(&self) -> NodeWeight {
        match *self {
            Node::NotYetTransmitted { .. } => NodeWeight(0),
            Node::Leaf { weight, .. } => weight,
            Node::Internal { weight, .. } => weight,
        }
    }
}

const MAX_SYMBOLS: usize = u8::MAX as usize + 1;

const MAX_NODES: usize = MAX_SYMBOLS * 2 - 1;

#[derive(/*Copy, Clone,*/ Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Huffman {
    tree: [Option<Node>; MAX_NODES],
    symbol_index: [Option<NodeIndex>; MAX_SYMBOLS],
    root: NodeIndex,
    nyt: NodeIndex,
    next: NodeIndex,
}

impl Huffman {
    pub fn new() -> Self {
        const NODE: Option<Node> = None;
        let mut tree = [NODE; MAX_NODES];

        let symbol_index = [None; MAX_SYMBOLS];

        tree[0] = Some(Node::NotYetTransmitted { parent: None });
        let nyt = NodeIndex(0);
        let root = nyt;

        Self {
            tree,
            symbol_index,
            root,
            nyt,
            next: NodeIndex(1),
        }
    }

    fn next(&mut self) -> NodeIndex {
        let next = self.next;
        self.next = NodeIndex(next.0 + 1);
        next
    }

    fn block_leader(&self, index: NodeIndex) -> NodeIndex {
        let mut i = index.0;
        let weight = self.tree[i].as_ref().unwrap().weight();
        while i >= 0 && self.tree[i].as_ref().unwrap().weight() == weight {
            if i == 0 {
                return NodeIndex(0);
            }
            i -= 1;
        }
        NodeIndex(i + 1)
    }

    fn swap_nodes(&mut self, node_index1: NodeIndex, node_index2: NodeIndex) {
        println!("swap {:?} ↔ {:?}", node_index1, node_index2);

        self.tree.swap(node_index1.0, node_index2.0);
        let [node1, .., node2] = &mut self.tree[node_index1.0..=node_index2.0] else {
            unreachable!()
        };
        match (node1, node2) {
            (
                Some(Node::NotYetTransmitted { parent: parent_1 }),
                Some(Node::NotYetTransmitted { parent: parent_2 }),
            ) => {
                (*parent_1, *parent_2) = (*parent_2, *parent_1);
            }
            (
                Some(Node::Leaf {
                    parent: parent_1, ..
                }),
                Some(Node::Leaf {
                    parent: parent_2, ..
                }),
            ) => {
                (*parent_1, *parent_2) = (*parent_2, *parent_1);
            }
            (
                Some(Node::Internal {
                    parent: parent_1, ..
                }),
                Some(Node::Internal {
                    parent: parent_2, ..
                }),
            ) => {
                (*parent_1, *parent_2) = (*parent_2, *parent_1);
            }
            _ => unreachable!(),
        }

        for (a_idx, b_idx) in [(node_index1, node_index2), (node_index2, node_index1)] {
            let a_node = self.tree[a_idx.0].as_ref().unwrap();
            let a_parent = if let Some(a_p) = a_node.parent_index() {
                self.tree[a_p.0].as_mut()
            } else {
                None
            };
            match a_parent {
                None => unreachable!(),
                Some(Node::NotYetTransmitted { .. }) => unreachable!(),
                Some(Node::Leaf { .. }) => unreachable!(),
                Some(Node::Internal { left, right, .. }) => {
                    if *left == b_idx {
                        *left = a_idx;
                    } else {
                        *right = a_idx;
                    }
                }
            }
        }
    }

    fn insert(&mut self, symbol: Symbol) {
        let symbol_index = self.symbol_index[symbol.0 as usize];
        println!("insert {:?} → {:?}", symbol, symbol_index);

        let mut node = if symbol_index.is_none() {
            let internal_index = self.nyt;
            let leaf_index = self.next();
            let nyt_index = self.next();

            let nyt_parent = self.tree[self.nyt.0].as_ref().unwrap().parent_index();

            let internal = Node::Internal {
                parent: nyt_parent,

                left: nyt_index,
                right: leaf_index,

                weight: NodeWeight(1),
            };

            let leaf = Node::Leaf {
                parent: internal_index,

                weight: NodeWeight(1),

                symbol,
            };

            let nyt = Node::NotYetTransmitted {
                parent: Some(internal_index),
            };

            self.symbol_index[symbol.0 as usize] = Some(leaf_index);

            self.tree[internal_index.0] = Some(internal);
            self.tree[leaf_index.0] = Some(leaf);
            self.tree[nyt_index.0] = Some(nyt);
            self.nyt = nyt_index;

            println!("inserted new nodes for symbol");
            self.print();

            nyt_parent
        } else {
            symbol_index
        };

        while let Some(node_index) = node {
            let node_parent = self.tree[node_index.0].as_ref().unwrap().parent_index();

            let leader = self.block_leader(node_index);
            println!("leader {:?}", leader);

            if leader != node_index && Some(leader) != node_parent {
                self.swap_nodes(node_index, leader);
                println!("swapped node and leader");
                self.print();
            }

            let n = self.tree[node_index.0].as_mut().unwrap();
            match n {
                Node::NotYetTransmitted { .. } => unreachable!(),
                Node::Leaf { weight, .. } => *weight = NodeWeight(weight.0 + 1),
                Node::Internal { weight, .. } => *weight = NodeWeight(weight.0 + 1),
            }
            println!("increased node weight");
            self.print();

            node = node_parent;
        }
    }

    fn print(&self) {
        println!("--- 🌳 ---");
        println!("root {:?}", self.root);
        println!("nyt {:?}", self.nyt);
        println!("next {:?}", self.next);
        println!();

        self.tree
            .iter()
            .enumerate()
            .filter(|(_i, n)| n.is_some())
            .for_each(|(i, n)| println!("tree {} → {:?}", i, n));
        println!();

        self.symbol_index
            .iter()
            .enumerate()
            .filter(|(_s, n)| n.is_some())
            .for_each(|(s, n)| println!("symbol {} → {:?}", s, n));

        println!("---");
        println!();
    }

    pub fn decode(&mut self, bits: &mut BitValIter<u8, Lsb0>, length: usize, bytes: &mut BytesMut) {
        println!("decode {:?} bytes", length);
        let mut node_index = self.root;
        let mut written = 0;
        while written < length {
            let node = self.tree[node_index.0].as_ref().unwrap();
            match *node {
                Node::NotYetTransmitted { .. } => {
                    let mut value = 0;
                    let b0 = bits.next().unwrap();
                    value &= (b0 as u8) << 0;
                    let b1 = bits.next().unwrap();
                    value &= (b1 as u8) << 1;
                    let b2 = bits.next().unwrap();
                    value &= (b2 as u8) << 2;
                    let b3 = bits.next().unwrap();
                    value &= (b3 as u8) << 3;
                    let b4 = bits.next().unwrap();
                    value &= (b4 as u8) << 4;
                    let b5 = bits.next().unwrap();
                    value &= (b5 as u8) << 5;
                    let b6 = bits.next().unwrap();
                    value &= (b6 as u8) << 6;
                    let b7 = bits.next().unwrap();
                    value &= (b7 as u8) << 7;

                    println!("decode NYT {:?}", value);
                    bytes.put_u8(value);
                    written += 1;
                    self.insert(Symbol(value));
                    node_index = self.root;
                }
                Node::Leaf { symbol, .. } => {
                    println!("decode leaf {:?}", symbol);
                    bytes.put_u8(symbol.0);
                    written += 1;
                    self.insert(symbol);
                    node_index = self.root;
                }
                Node::Internal { left, right, .. } => {
                    let bit = bits.next().unwrap();
                    println!("decode bit {:?}", bit);
                    node_index = if bit { right } else { left };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::slice::BitSlice;

    #[test]
    fn huffman_new() {
        let mut huff = Huffman::new();
        let encoded_bytes = b"\x00\xFF";
        let encoded_bits = BitSlice::<_, Lsb0>::from_slice(encoded_bytes);
        let decoded_len = 4;
        let mut decoded_bytes = BytesMut::new();

        println!("initial tree");
        huff.print();

        huff.decode(
            &mut encoded_bits.iter().by_vals(),
            decoded_len,
            &mut decoded_bytes,
        );

        assert_eq!(&decoded_bytes[..], b"");
    }
}
