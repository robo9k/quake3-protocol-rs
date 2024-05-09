#![no_main]

use quake3_huffman::Huffman;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut huff = Huffman::adaptive();
    // with arbitrary data we don't know if it decompresses and to how much
    let len = data.len() * 2;
    let mut decoded_bytes = bytes::BytesMut::new();

    // swallow Err, don't panic
    let _ = huff.decode(data, len, &mut decoded_bytes);
});
