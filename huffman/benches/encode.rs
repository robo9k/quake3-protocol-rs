use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use quake3_huffman::Huffman;

pub fn bench_adaptive(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");

    let userinfo = b"\"\\challenge\\-9938504\\qport\\2033\\protocol\\68\\name\\UnnamedPlayer\\rate\\25000\\snaps\\20\\model\\sarge\\headmodel\\sarge\\team_model\\james\\team_headmodel\\*james\\color1\\4\\color2\\5\\handicap\\100\\sex\\male\\cl_anonymous\\0\\cg_predictItems\\1\\teamtask\\0\\cl_voipProtocol\\opus\\cl_guid\\D17466611282F45B65CE2FD80F83B6B0\"";

    group.throughput(Throughput::Bytes(userinfo.len() as u64));

    group.bench_with_input(
        BenchmarkId::new("adaptive", "default userinfo"),
        userinfo,
        |b, i| {
            b.iter(|| {
                // can't reuse the codec in adaptive mode, so setup is part of the benchmark
                let mut huff = Huffman::adaptive();

                let _ = huff.encode(i);
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench_adaptive);
criterion_main!(benches);
