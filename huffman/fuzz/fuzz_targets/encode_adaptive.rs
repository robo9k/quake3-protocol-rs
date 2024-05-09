#![no_main]

use quake3_huffman::Huffman;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut huff = Huffman::adaptive();

    let _ = huff.encode(data);
});
