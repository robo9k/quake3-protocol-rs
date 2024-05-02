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

    fn insert(&mut self, symbol: Symbol) {
        let symbol_index = self.symbol_index[symbol.0 as usize];

        let mut node = if symbol_index.is_none() {
            let internal_index = self.next();
            let leaf_index = self.next();

            let nyt_parent = self.tree[self.nyt.0].as_ref().unwrap().parent_index();

            println!("nyt: {:?}", self.nyt);
            println!("internal: {:?}", internal_index);
            println!("leaf: {:?}", leaf_index);
            println!();

            let internal = Node::Internal {
                parent: nyt_parent,

                left: self.nyt,
                right: leaf_index,

                weight: NodeWeight(1),
            };

            let leaf = Node::Leaf {
                parent: internal_index,

                weight: NodeWeight(1),

                symbol,
            };

            let internal_parent = internal.parent_index();
            if let Some(parent) = internal_parent {
                let internal_parent = self.tree[parent.0].as_mut().unwrap();
                match internal_parent {
                    Node::NotYetTransmitted { .. } => unreachable!(),
                    Node::Leaf { .. } => unreachable!(),
                    Node::Internal { left, .. } => *left = internal_index,
                }
            } else {
                self.root = internal_index;
            }

            match self.tree[self.nyt.0].as_mut().unwrap() {
                Node::NotYetTransmitted { parent } => *parent = Some(internal_index),
                Node::Leaf { .. } => unreachable!(),
                Node::Internal { .. } => unreachable!(),
            }

            self.symbol_index[symbol.0 as usize] = Some(leaf_index);

            self.tree[leaf_index.0] = Some(leaf);
            self.tree[internal_index.0] = Some(internal);

            self.print();

            if let Some(parent) = internal_parent {
                Some(parent)
            } else {
                Some(leaf_index)
            }
        } else {
            None
        };

        while let Some(node_index) = node {
            // TODO: find node block leader
            // TODO: if leader is neither node nor parent, swap node and leader
            todo!();

            let n = self.tree[node_index.0].as_mut().unwrap();
            match n {
                Node::NotYetTransmitted { .. } => unreachable!(),
                Node::Leaf { weight, .. } => *weight = NodeWeight(weight.0 + 1),
                Node::Internal { weight, .. } => *weight = NodeWeight(weight.0 + 1),
            }

            node = n.parent_index();
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
        let mut node_index = self.root;
        let mut written = 0;
        while written < length {
            let node = self.tree[node_index.0].as_ref().unwrap();
            match *node {
                Node::NotYetTransmitted { .. } => {
                    let value = 0x0; // TODO: read and reverse u8 from bits
                    bytes.put_u8(value);
                    written += 1;
                    self.insert(Symbol(value));
                    node_index = self.root;
                }
                Node::Leaf { symbol, .. } => {
                    bytes.put_u8(symbol.0);
                    written += 1;
                    self.insert(symbol);
                    node_index = self.root;
                }
                Node::Internal { left, right, .. } => {
                    node_index = if bits.next().unwrap() { right } else { left };
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
        let mut decoded_bytes = BytesMut::new();

        huff.decode(
            &mut encoded_bits.iter().by_vals(),
            encoded_bytes.len(),
            &mut decoded_bytes,
        );
    }
}
