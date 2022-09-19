use std::{fs};
use std::path::{PathBuf};

use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use clap::{Parser, Subcommand};
use env_logger::Env;
use itertools::Itertools;
use indicatif::ProgressIterator;
use rusqlite::Connection;

use tenhou_db::{
    db::*,
    download::*,
    parse::*,
    extract::*,
};

static DEFAULT_DB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/tenhou.sqlite");

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Args {
    #[clap(long, value_parser, value_name = "FILE")]
    db: Option<PathBuf>,

    #[clap(long)]
    three: Option<bool>,

    #[clap(long, action)]
    redownload: bool,

    #[clap(subcommand)]
    command: Option<Command>
}

#[derive(Subcommand, Debug)]
enum Command {
    Recent,
    File {
        #[clap(value_parser, value_name = "FILE")]
        files: Vec<PathBuf>,
    }
}

#[derive(Debug)]
struct Filter {
    three: Option<bool>,
}

fn main() -> Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args: Args = dbg!(Args::parse());

    let db_file = args.db.unwrap_or(PathBuf::from(&DEFAULT_DB_PATH));
    log::info!("Using DB file: {}", db_file.to_str().unwrap());
    let db = init_db(&db_file)?;

    let filter = Filter {
        three: args.three,
    };
    log::info!("Filter: {:?}", filter);

    match args.command {
        Some(Command::Recent) => {
            for archive in list_recent_archives()? {
                if let Err(e) = download_and_process_archive(&db, &archive, &filter, args.redownload) {
                    log::warn!("Failed to load '{}' => {:#}", archive.url, e);
                }
            }
        }
        Some(Command::File{ files }) => {
            for path in files {
                let path = Utf8Path::from_path(&path).context("Path is not UTF-8")?;
                if let Err(e) = load_file(&db, &path, &filter, args.redownload) {
                    log::warn!("Failed to load file '{}' => {:#}", path, e);
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn download_and_process_archive(db: &Connection, archive: &RemoteArchiveFile, filter: &Filter, redownload: bool) -> Result<()> {
    log::debug!("Download and process archive: {}", archive.url);
    let mut should_download = true;
    if !redownload {
        if is_archive_in_db(db, &archive.name)? {
            log::debug!("Archive {} exists in DB; skipping.", archive.name);
            should_download = false;
        }
    }
    if should_download {
        let entries = download_archive(&archive)?
            .split("\n")
            .filter_map(parse_archive_line)
            .collect_vec();
        for entry in &entries {
            add_game_meta_only(db, entry)?;
        }
        let mut overall_result = Ok(());
        download_and_add_game_json_batch(
            db,
            entries.iter().progress(),
            filter,
            redownload,
            &mut overall_result);
        add_archive(
            db,
            &archive.name,
            Some(&archive.url),
            parse_gz_date_hour(&archive.name),
        )?;
        overall_result
    } else {
        // TODO(summivox): how about we still see if the date range represented by this archive
        //     is appropriately downloaded?
        Ok(())
    }
}

fn load_file(db: &Connection, path: &Utf8Path, filter: &Filter, redownload: bool) -> Result<()> {
    log::debug!("load_file({})", path);
    let file = fs::File::open(path.as_std_path())?;
    let mut overall_result = Ok(());
    match path.extension() {
        Some("zip") => {
            process_zip_file(file, |name, batch| {
                add_archive(db, name, None, parse_gz_date_hour(name)).unwrap();
                for entry in batch {
                    add_game_meta_only(db, entry).unwrap();
                }
                download_and_add_game_json_batch(
                    db,
                    batch,
                    filter,
                    redownload,
                    &mut overall_result,
                );
            })?;
        }
        Some(_) => {
            let filename = path.file_name().context("Bad filename")?;
            add_archive(db, filename, None, parse_gz_date_hour(filename)).unwrap();

            let html = read_html_maybe_gz(path, file)?;
            let mut overall_result = Ok(());
            let entries = html.split("\n").filter_map(parse_archive_line).collect_vec();
            for entry in &entries {
                add_game_meta_only(db, entry)?;
            }
            download_and_add_game_json_batch(
                db,
                entries.iter(),
                filter,
                redownload,
                &mut overall_result,
            );
        }
        _ => bail!("Not recognized file extension")
    }
    overall_result
}

fn download_and_add_game_json(db: &Connection, entry: &GameLogInfo) -> Result<()> {
    download_game_json(&entry.id)
        .context("Failed to download game")
        .and_then(|game_json|
            add_game_json_for_id(db, &entry.id, &game_json)
                .context("Failed to add game to DB"))?;
    Ok(())
}

fn download_and_add_game_json_batch<'a>(
    db: &Connection,
    batch: impl IntoIterator<Item=&'a GameLogInfo>,
    filter: &Filter,
    redownload: bool,
    overall_result: &mut Result<()>,
) {
    for entry in batch {
        if let Some(rule_id) = entry.rule_id {
            let rule_parts = parse_rule_id(rule_id);
            if let Some(three) = filter.three {
                if three != rule_parts.three {
                    continue;
                }
            }
        }

        let mut should_download = true;
        if !redownload {
            match get_db_game_by_id(db, &entry.id) {
                Ok((_, Some(_))) => {
                    log::info!("Game '{}' exists in DB; will not redownload.", entry.id);
                    should_download = false;
                }
                Err(e) => {
                    log::warn!("get_db_game_by_id('{}') => {:#}", entry.id, e);
                }
                _ => {}
            }
        }

        if should_download {
            let entry_result = download_and_add_game_json(db, entry);
            if let Err(e) = &entry_result {
                log::warn!("Download and process game id '{}' => {:#}", entry.id, e);
            }
            replace_with::replace_with(
                overall_result,
                || Ok(()),
                |result| result.or(entry_result));
        }
    }
}
