use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use quake3_huffman::Huffman;

pub fn bench_adaptive(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    let userinfo = hex_literal::hex!(
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
    let len = 0x0128;

    group.throughput(Throughput::Bytes(userinfo.len() as u64));

    group.bench_with_input(
        BenchmarkId::new("adaptive", "default userinfo"),
        &(&userinfo[..], len),
        |b, i| {
            b.iter(|| {
                // can't reuse the codec in adaptive mode, so setup is part of the benchmark
                let mut huff = Huffman::adaptive();
                let mut decoded_bytes = bytes::BytesMut::new();

                let _ = huff.decode(i.0, i.1, &mut decoded_bytes);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench_adaptive);
criterion_main!(benches);
