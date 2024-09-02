use std::fs::{read_to_string, write};
use std::path::{absolute, Path, PathBuf};

use clap::Parser;
use glob::glob;
use itertools::Itertools;
use sqruff_lib::core::config::{FluffConfig, Value};
use sqruff_lib::core::linter::linter::Linter;
use sqruff_lib::core::parser::segments::base::Tables;

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[arg(short, long, default_value = "*.sql")]
    filter: String,

    #[arg(short, long, default_value = "**")]
    dialect: String,
}

#[derive(Debug)]
struct ScriptPack {
    dialect: String,

    files: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let all_packs = gather_files(&args.filter, &args.dialect);
    let files_found = all_packs.iter().fold(0, |acc, p| acc + p.files.len());

    println!("Using the following settings: {:?}", args);
    println!("Found {:?} scripts to generate.", files_found);

    for pack in all_packs {
        println!("Generating fixtures for {:?}", pack.dialect);
        generate_fixtures(&pack);
    }
}

fn gather_files(filter: &String, dialect: &String) -> Vec<ScriptPack> {
    let search_path = format!("crates/lib/test/fixtures/dialects/{}/{}", dialect, filter);

    glob(search_path.as_str())
        .unwrap()
        .flatten()
        .chunk_by(|p| PathBuf::from(p.parent().unwrap()))
        .into_iter()
        .map(|(path, files)| ScriptPack {
            dialect: String::from(path.file_name().unwrap().to_str().unwrap()),
            files: files.map(|f| absolute(f).unwrap()).collect_vec(),
        })
        .collect()
}

fn generate_fixtures(pack: &ScriptPack) {
    let config = FluffConfig::new(
        [(
            "core".into(),
            Value::Map([("dialect".into(), Value::String(pack.dialect.as_str().into()))].into()),
        )]
        .into(),
        None,
        None,
    );

    let dialect = PathBuf::from(&pack.dialect);
    let linter = Linter::new(config.clone(), None, None);

    for file in pack.files.iter() {
        println!(
            "Generating fixture for {}/{}",
            dialect.display(),
            Path::new(file.file_name().unwrap()).display()
        );

        let target_file = file.with_extension("yml");
        let sql = read_to_string(file).unwrap();
        let tables = Tables::default();

        let tree = linter
            .parse_string(&tables, &sql, None, None, None)
            .unwrap()
            .tree
            .unwrap()
            .to_serialised(true, true);

        let yaml = serde_yaml::to_string(&tree).unwrap().replace("\n", "\r\n");

        write(target_file, yaml).unwrap()
    }
}
