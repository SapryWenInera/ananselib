use {
    criterion::{criterion_group, criterion_main, BenchmarkId, Criterion},
    fastsearch::FastSearch,
    rand::{thread_rng, Rng},
};

fn populate_vec(vec: &mut Vec<u8>, size: usize) {
    let mut rng = thread_rng();
    for _ in 0..size {
        vec.push(rng.gen::<u8>())
    }
}

fn naive_rsearch(buffer: &[u8], signature: &[u8]) -> Option<usize> {
    'outer: for index in (0..buffer.len()).rev() {
        for (signature_index, signature_byte) in signature.iter().rev().enumerate() {
            if let Some(next_index) = index.checked_sub(signature_index) {
                if buffer[next_index] != *signature_byte {
                    continue 'outer;
                }
            } else {
                break 'outer;
            }
        }
        return Some(index);
    }
    None
}

fn rs_async_zip_search(size: u64) {
    let mut haystack = Vec::new();
    let mut neddle = Vec::new();

    let hay_size = (size + 4) as usize;
    let neddle_size = 4;
    populate_vec(&mut haystack, hay_size);
    populate_vec(&mut neddle, neddle_size);

    let _ = haystack
        .chunks(2048)
        .find_map(|chunk| naive_rsearch(chunk, &neddle));
}

fn rsearch(size: u64) {
    let mut haystack = Vec::new();
    let mut neddle = Vec::new();

    let hay_size = (size + 4) as usize;
    let neddle_size = 4;
    populate_vec(&mut haystack, hay_size);
    populate_vec(&mut neddle, neddle_size);

    let _ = haystack.rsearch(&neddle);
}

fn bench(bench: &mut Criterion) {
    let mut group = bench.benchmark_group("Search");
    for i in [300, 6000] {
        let id = format!("search with {}", i);
        group.bench_with_input(BenchmarkId::new("Naive Search", &id), &i, |b, i| {
            b.iter(|| rs_async_zip_search(*i))
        });
        group.bench_with_input(
            BenchmarkId::new("Boyer-Moore-Horspool Search", &id),
            &i,
            |b, i| b.iter(|| rsearch(*i)),
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
