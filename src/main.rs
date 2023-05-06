use std::{path::PathBuf, str::FromStr};

use crate::store::backup::Backup;

mod store;

fn main() {
    let backup = Backup::load(PathBuf::from_str("F:\\neos backup 2").unwrap()).unwrap();
    let out = std::fs::File::create("dump.json").unwrap();
    serde_json::to_writer_pretty(out, &backup).unwrap();

    return;

    let interner_ = store::internment::Interner::default();

    let mut map = std::collections::BTreeMap::new();
    let mut count = 0;
    for path in std::fs::read_dir("F:\\neos backup 2\\U-Earthmark\\Records\\").unwrap() {
        let path = path.unwrap().path();

        let content = std::fs::File::open(path).unwrap();
        let buf_content = std::io::BufReader::new(content);
        let manifest: store::backup::Record = serde_json::from_reader(buf_content).unwrap();

        if manifest.record_type == store::backup::RecordType::Directory
            && manifest.asset_uri != None
        {
            println!("{:#?}", manifest);
        }
        map.insert(manifest.record_type.clone(), manifest);
        count += 1;
    }

    println!("{} total entries", count);
    let out = std::fs::File::create("dump.json").unwrap();
    serde_json::to_writer_pretty(out, &map).unwrap();
}
