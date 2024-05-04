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
    fn parent(&self) -> Option<NodeIndex> {
        match *self {
            Node::NotYetTransmitted { parent } => parent,
            Node::Leaf { parent, .. } => Some(parent),
            Node::Internal { parent, .. } => parent,
        }
    }

    fn set_parent(&mut self, index: NodeIndex) {
        match self {
            Node::NotYetTransmitted { parent } => *parent = Some(index),
            Node::Leaf { parent, .. } => *parent = index,
            Node::Internal { parent, .. } => *parent = Some(index),
        }
    }

    fn weight(&self) -> NodeWeight {
        match *self {
            Node::NotYetTransmitted { .. } => NodeWeight(0),
            Node::Leaf { weight, .. } => weight,
            Node::Internal { weight, .. } => weight,
        }
    }

    fn increase_weight(&mut self) {
        match self {
            Node::NotYetTransmitted { .. } => unreachable!(),
            Node::Leaf { weight, .. } => *weight = NodeWeight(weight.0 + 1),
            Node::Internal { weight, .. } => *weight = NodeWeight(weight.0 + 1),
        }
    }
}

const MAX_SYMBOLS: usize = u8::MAX as usize + 1;

const MAX_NODES: usize = MAX_SYMBOLS * 2 - 1;

#[derive(/*Copy, Clone,*/ Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Huffman {
    tree: [Option<Node>; MAX_NODES],
    symbol_index: [Option<NodeIndex>; MAX_SYMBOLS],
    nyt: NodeIndex,
    next: NodeIndex,
}

impl Huffman {
    const ROOT: NodeIndex = NodeIndex(0);

    pub fn new() -> Self {
        const NODE: Option<Node> = None;
        let mut tree = [NODE; MAX_NODES];

        let symbol_index = [None; MAX_SYMBOLS];

        tree[Self::ROOT.0] = Some(Node::NotYetTransmitted { parent: None });
        let nyt = Self::ROOT;

        Self {
            tree,
            symbol_index,
            nyt,
            next: NodeIndex(1),
        }
    }

    fn next(&mut self) -> NodeIndex {
        let next = self.next;
        self.next = NodeIndex(next.0 + 1);
        next
    }

    fn node_ref(&self, index: NodeIndex) -> &Node {
        self.tree[index.0].as_ref().unwrap()
    }

    fn node_mut(&mut self, index: NodeIndex) -> &mut Node {
        self.tree[index.0].as_mut().unwrap()
    }

    fn block_leader(&self, index: NodeIndex) -> NodeIndex {
        let mut i = index.0;
        let weight = self.node_ref(index).weight();
        while i >= 0 && self.node_ref(NodeIndex(i)).weight() == weight {
            if i == 0 {
                return NodeIndex(0);
            }
            i -= 1;
        }
        NodeIndex(i + 1)
    }

    fn swap_nodes(&mut self, a: NodeIndex, b: NodeIndex) {
        println!("swap nodes {} ↔ {}", a.0, b.0);

        let a_parent = self.node_ref(a).parent().unwrap();
        let b_parent = self.node_ref(b).parent().unwrap();

        self.tree.swap(a.0, b.0);
        self.node_mut(a).set_parent(a_parent);
        self.node_mut(b).set_parent(b_parent);

        match self.node_ref(a) {
            &Node::NotYetTransmitted { .. } => unreachable!(),
            &Node::Leaf { symbol, .. } => {
                self.symbol_index[symbol.0 as usize] = Some(a);
            }
            &Node::Internal { left, right, .. } => {
                self.node_mut(left).set_parent(a);
                self.node_mut(right).set_parent(a);
            }
        }
        match self.node_ref(b) {
            &Node::NotYetTransmitted { .. } => unreachable!(),
            &Node::Leaf { symbol, .. } => {
                self.symbol_index[symbol.0 as usize] = Some(b);
            }
            &Node::Internal { left, right, .. } => {
                self.node_mut(left).set_parent(b);
                self.node_mut(right).set_parent(b);
            }
        }
    }

    fn insert(&mut self, symbol: Symbol) {
        let symbol_index = self.symbol_index[symbol.0 as usize];
        println!("insert symbol {} → {:?}", symbol.0, symbol_index);

        let mut node = if symbol_index.is_none() {
            let internal_index = self.nyt;
            let leaf_index = self.next();
            let nyt_index = self.next();

            let nyt_parent = self.node_ref(self.nyt).parent();

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
            self.graphviz();

            nyt_parent
        } else {
            symbol_index
        };

        while let Some(node_index) = node {
            let node_parent = self.node_ref(node_index).parent();

            let leader = self.block_leader(node_index);
            println!("leader for {}: {}", node_index.0, leader.0);

            if leader != node_index && Some(leader) != node_parent {
                self.swap_nodes(node_index, leader);
                println!("swapped node {} and leader {}", node_index.0, leader.0);
                self.graphviz();
            }

            self.node_mut(node_index).increase_weight();
            println!("increased node {} weight", node_index.0);
            self.graphviz();

            node = node_parent;
        }
    }

    fn print(&self) {
        println!("--- 🌳 ---");
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

    fn graphviz(&self) {
        println!("digraph {{");

        // graph attributes
        println!("\tordering=out");

        // node attributes
        println!("\t{{");
        self.tree
            .iter()
            .enumerate()
            .filter(|(_i, n)| n.is_some())
            .for_each(|(i, n)| {
                let node = n.as_ref().unwrap();
                let shape = match node {
                    Node::NotYetTransmitted { .. } => "Mrecord",
                    Node::Leaf { .. } => "Mrecord",
                    Node::Internal { .. } => "record",
                };
                let label = match node {
                    Node::NotYetTransmitted { .. } => format!("<id>@{}|<weight>0|<symbol>NYT", i),
                    Node::Leaf { weight, symbol, .. } => {
                        format!("<id>@{}|<weight>{}|<symbol>{:#04X}", i, weight.0, symbol.0)
                    }
                    Node::Internal { weight, .. } => {
                        format!("<id>@{}|<weight>{}|<symbol>-", i, weight.0)
                    }
                };
                let style = match node {
                    Node::NotYetTransmitted { .. } => "style=dashed,",
                    Node::Leaf { .. } => "",
                    Node::Internal { .. } => {
                        if Self::ROOT.0 == i {
                            "style=bold,"
                        } else {
                            ""
                        }
                    }
                };
                println!("\t\t{} [shape={},{}label=\"{}\"]", i, shape, style, label);
            });
        println!("\t}}");

        // nodes
        self.tree
            .iter()
            .enumerate()
            .filter(|(_i, n)| n.is_some())
            .for_each(|(i, n)| {
                let node = n.as_ref().unwrap();
                match node {
                    Node::Internal { left, right, .. } => {
                        println!("\t{} -> {}:id [label=0]", i, left.0);
                        println!("\t{} -> {}:id [label=1]", i, right.0);
                    }
                    _ => {}
                }
            });

        println!("}}");
        println!();
    }

    pub fn decode(&mut self, bits: &mut BitValIter<u8, Lsb0>, length: usize, bytes: &mut BytesMut) {
        println!("decode {} bytes", length);
        let mut node_index = Self::ROOT;
        let mut written = 0;
        while written < length {
            let node = self.node_ref(node_index);
            match *node {
                Node::NotYetTransmitted { .. } => {
                    let mut value = 0;
                    let b0 = bits.next().unwrap();
                    value |= (b0 as u8) << 0;
                    let b1 = bits.next().unwrap();
                    value |= (b1 as u8) << 1;
                    let b2 = bits.next().unwrap();
                    value |= (b2 as u8) << 2;
                    let b3 = bits.next().unwrap();
                    value |= (b3 as u8) << 3;
                    let b4 = bits.next().unwrap();
                    value |= (b4 as u8) << 4;
                    let b5 = bits.next().unwrap();
                    value |= (b5 as u8) << 5;
                    let b6 = bits.next().unwrap();
                    value |= (b6 as u8) << 6;
                    let b7 = bits.next().unwrap();
                    value |= (b7 as u8) << 7;

                    println!("decode NYT {:#04X}", value);
                    bytes.put_u8(value);
                    written += 1;
                    self.insert(Symbol(value));
                    node_index = Self::ROOT;
                }
                Node::Leaf { symbol, .. } => {
                    println!("decode leaf {:#04X}", symbol.0);
                    bytes.put_u8(symbol.0);
                    written += 1;
                    self.insert(symbol);
                    node_index = Self::ROOT;
                }
                Node::Internal { left, right, .. } => {
                    let bit = bits.next().unwrap();
                    println!("decode bit {}", bit);
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
        // this is from a wireshark dump
        let encoded_bytes = hex_literal::hex!(
            "
            44 74 30 8e 05 0c c7 26 
            c3 14 ec 8e f9 67 d0 1a 4e 29 98 01 c7 c3 7a 30 
            2c 2c 19 1c 13 87 c2 de 71 0a 5c ac 30 cd 40 ce 
            3a ca af 96 2a b0 d9 3a b7 b0 fd 4d a8 0e c9 ba 
            79 4c 28 0a c4 0a 4f 83 02 9b 9f 69 e4 0a c3 38 
            47 9b cf 22 af 61 f6 64 6f 13 7c a3 ae 1f af 06 
            52 b7 3c a3 06 5f 3a f4 8f 66 d2 40 ac ee 2b 2d 
            ea 38 18 f9 b7 f2 36 37 80 ea 17 e9 d5 40 58 f7 
            0f c6 b2 3a 85 e5 bb ca f7 78 77 09 2c e1 e5 7b 
            cc ad 59 0f 3c ea 67 2a 37 1a 31 c7 83 e5 02 d7 
            d1 dd c0 73 eb e6 5d 4c 32 87 a4 a4 8d 2e 1b 08 
            0b 38 11 ac 7b 9a 34 16 e2 e6 d1 3b f0 f8 f2 99 
            da c4 91 b7 4b 53 cf 82 a6 da 10 61 89 b0 5b 6c 
            6e c3 46 e3 b7 7c 19 62 38 ac 42 48 23 ab 11 e6 
            20 0a b8 75 91 26 12 6e 92 25 65 c9 00       
        "
        );
        let encoded_bits = BitSlice::<_, Lsb0>::from_slice(&encoded_bytes);
        // there's a u16 after "connect "
        let decoded_len = 0x0128;
        let mut decoded_bytes = BytesMut::new();

        println!("initial tree");
        huff.graphviz();

        huff.decode(
            &mut encoded_bits.iter().by_vals(),
            decoded_len,
            &mut decoded_bytes,
        );

        let decoded = std::str::from_utf8(&decoded_bytes[..]).unwrap();
        println!("decoded: {:?}", decoded);

        // this is taken from debug logs and not debugger/packets, so might be incorrect
        let expected = b"n\\UnnamedPlayer\\t\\0\\model\\sarge\\hmodel\\sarge\\g_redteam\\\\g_blueteam\\\\c1\\4\\c2\\5\\hc\\100\\w\\0\\l\\0\\tt\\0\\tl\\0";

        assert_eq!(&decoded_bytes[..], expected);
    }
}
