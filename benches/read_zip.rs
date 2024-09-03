use {
    async_zip::base::read::seek::ZipFileReader,
    criterion::{
        async_executor::SmolExecutor, criterion_group, criterion_main, BenchmarkId, Criterion,
    },
    libzip_rs::{error::ZipResult, ZipArchive as AsyncZipArchive},
    smol::{fs::File as AsyncFile, io::BufReader, stream::StreamExt},
    std::{
        fs::{read_dir, File},
        io::Read,
        path::{Path, PathBuf},
    },
    zip::ZipArchive,
};

fn recursive_read<P>(path: P) -> ZipResult<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let root_path = path.as_ref();
    let mut paths = Vec::new();
    if root_path.is_dir() {
        let dir = read_dir(root_path)?;

        for entry in dir {
            let entry = entry.ok();
            if let Some(entry) = entry {
                let mut recurse = recursive_read(entry.path())?;
                paths.append(&mut recurse)
            } else {
                break;
            }
        }
    } else {
        paths.push(root_path.into())
    }
    Ok(paths)
}

async fn read_libzip<P>(path: P)
where
    P: AsRef<Path>,
{
    let mut reader = AsyncFile::open(path).await.unwrap();
    let mut archive = AsyncZipArchive::new(&mut reader).await.unwrap();
    let mut buffer = Vec::with_capacity(archive.len());
    let mut iter = archive.stream();

    while let Some(file) = iter.next().await {
        let file = file.unwrap();
        if file.is_file() {
            let file = file.extract().await;
            let data = file.unwrap();
            buffer.push(data);
            break;
        }
    }
}

async fn read_async_zip<P>(path: P)
where
    P: AsRef<Path>,
{
    let mut buffer = Vec::new();
    let mut reader = BufReader::new(AsyncFile::open(path).await.unwrap());
    let mut zip = ZipFileReader::new(&mut reader).await.unwrap();

    for idx in 0..zip.file().entries().len() {
        let mut data = Vec::new();
        let entry = zip.file().entries().get(idx).unwrap();

        if !entry.dir().unwrap() {
            let mut reader = zip.reader_without_entry(idx).await.unwrap();
            smol::io::copy(&mut reader, &mut data).await.unwrap();
            buffer.push(data);
            break;
        }
    }
}

fn read_zip2<P>(path: P)
where
    P: AsRef<Path>,
{
    let mut reader = File::open(path).unwrap();
    let mut buffer = Vec::new();
    let mut archive = ZipArchive::new(&mut reader).unwrap();
    let files: Vec<String> = archive
        .file_names()
        .filter_map(|c| {
            if !c.ends_with('/') {
                Some(String::from(&*c))
            } else {
                None
            }
        })
        .collect();
    let mut files = files.into_iter();

    while let Some(file) = files.next() {
        let mut data = Vec::new();
        let mut f = archive.by_name(&file).unwrap();
        f.read_to_end(&mut data).unwrap();
        buffer.push(data);
        break;
    }
}

fn read(b: &mut Criterion) {
    let path = "zip/tests";
    let paths = recursive_read(path).unwrap();

    let mut group = b.benchmark_group("Zip Read Archive");
    group.sample_size(10);

    for path in paths {
        let id = format!("Reading zip: {:?}", path);

        group.bench_with_input(
            BenchmarkId::new("LibAnanse Zip", &id),
            &path,
            |bench, path| bench.to_async(SmolExecutor).iter(|| read_libzip(path)),
        );
        group.bench_with_input(BenchmarkId::new("Async_zip", &id), &path, |bench, path| {
            bench.to_async(SmolExecutor).iter(|| read_async_zip(path))
        });
        group.bench_with_input(BenchmarkId::new("Zip2", &id), &path, |bench, path| {
            bench.iter(|| read_zip2(path))
        });
    }
    group.finish();
}

criterion_group!(benches, read);
criterion_main!(benches);
