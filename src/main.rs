use std::{fs, io::Write, path::PathBuf, str::FromStr, rc::Rc};

use bson::Bson;
use serde::de::DeserializeOwned;
use store::backup::SZBson;

use crate::store::backup::{AssetUri, Backup, Manifest, WellKnownAssetKind};

mod store;

fn main() {
    let bson: Manifest = read_7zbson("fed049610a4bd07198d82367bb16c106217e240fa21c34256a9454e824a0cc5a").unwrap();
    writeln!(fs::File::create("dump.ron").unwrap(), "{:#?}", bson).unwrap();

    scan_for_invalid();
}

fn read_7zbson<Output : DeserializeOwned>(asset: &str) -> Result<Output, store::backup::Error> {
    let b = Backup {
        assets_dir: "F:\\neos backup 2\\Assets".into(),
        ..Default::default()
    };

    let asset = SZBson(Rc::new(asset.into()));

    asset.open(&b)
}

fn scan_for_invalid() {
    println!("Parsing backup...");
    let backup = Backup::load(PathBuf::from_str("F:\\neos backup 2").unwrap()).unwrap();
    println!("Parsing backup. done!");

    println!("Scanning assets...");
    for val in backup.accounts.values() {
        for rec in val.records.values() {
            if let Some(AssetUri::SZBson(asset)) = &rec.asset_uri {
                println!("Opening {:?}", asset);
                let res: Result<Manifest, _> = asset.open(&backup);
                if let Err(e) = res {
                    println!(
                        "Error parsing {:?}, dumping to dump.ron: {:#?}",
                        rec.asset_uri, e
                    );
                    let res: Bson = asset.open(&backup).unwrap();
                    writeln!(fs::File::create("dump.ron").unwrap(), "{:#?}", res).unwrap();
                    return;
                }
            }
        }
    }
}