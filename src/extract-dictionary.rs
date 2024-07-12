// Wordroute – A word game
// Copyright (C) 2024  Neil Roberts
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::process::ExitCode;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, BufReader, Write};
use std::fs::File;
use std::ffi::OsString;

use clap::Parser;

#[derive(Parser)]
#[command(name = "Build")]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    dictionary: OsString,
    #[arg(short, long, value_name = "FILE")]
    bonus_words: OsString,
    #[arg(short, long, value_name = "FILE")]
    readlex: OsString,
    #[arg(short, long, value_name = "LENGTH", default_value_t = 4)]
    minimum_length: usize,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "Shaw")]
    shavian: String,
    pos: String,
    var: String,
}

static BANNED_POSITIONS: [&'static str; 1] = [
    "NP0",
];

static ALLOWED_VARIATIONS: [&'static str; 1] = [
    "RRP",
];

type ReadLexMap = HashMap<String, Vec<Entry>>;

fn is_shavian(s: &str) -> bool {
    s.chars().all(|ch| ch >= '𐑐' && ch <= '𐑿')
}

fn write_dictionaries<D, B>(
    mut dictionary: D,
    mut bonus_words: B,
    map: ReadLexMap,
    minimum_length: usize,
) -> Result<(), std::io::Error>
    where D: Write,
          B: Write
{
    let mut all_words = HashSet::new();
    let mut allowed_words = HashSet::new();

    for (_, entries) in map.into_iter() {
        for entry in entries.into_iter() {
            if BANNED_POSITIONS.iter().find(|&p| p == &entry.pos).is_some() ||
                entry.shavian.chars().count() < minimum_length ||
                !is_shavian(&entry.shavian)
            {
                continue;
            }

            // Anything that’s not one of the chosen variations is
            // considered a bonus word
            if ALLOWED_VARIATIONS.iter().find(|&p| p == &entry.var).is_some() {
                allowed_words.insert(entry.shavian.clone());
            }

            all_words.insert(entry.shavian);
        }
    }

    let mut all_words = all_words.into_iter().collect::<Vec<_>>();
    all_words.sort_unstable();

    for word in all_words.into_iter() {
        if !allowed_words.contains(&word) {
            writeln!(&mut bonus_words, "{}", &word)?;
        }

        writeln!(&mut dictionary, "{}", word)?;
    }

    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let map = match File::open(&cli.readlex)
        .map_err(|e| e.to_string())
        .and_then(|file| {
            serde_json::from_reader::<_, ReadLexMap>(BufReader::new(file))
                .map_err(|e| e.to_string())
        })
    {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}: {}", cli.readlex.to_string_lossy(), e);
            return ExitCode::FAILURE;
        },
    };

    let dictionary = match File::create(&cli.dictionary) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}", cli.dictionary.to_string_lossy(), e);
            return ExitCode::FAILURE;
        },
    };

    let dictionary = BufWriter::new(dictionary);

    let bonus_words = match File::create(&cli.bonus_words) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}", cli.bonus_words.to_string_lossy(), e);
            return ExitCode::FAILURE;
        },
    };

    let bonus_words = BufWriter::new(bonus_words);

    if let Err(e) = write_dictionaries(
        dictionary,
        bonus_words,
        map,
        cli.minimum_length,
    ) {
        eprintln!("{}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_shavian() {
        assert!(is_shavian("𐑐𐑑𐑒𐑓𐑔𐑕𐑖𐑗𐑘𐑙𐑚𐑛𐑜𐑝𐑞𐑟𐑠𐑡𐑢𐑣𐑤𐑥𐑦𐑧𐑨𐑩𐑪𐑫𐑬𐑭𐑮𐑯𐑰𐑱𐑲𐑳𐑴𐑵𐑶𐑷𐑸𐑹𐑺𐑻𐑼𐑽𐑾𐑿"));
        assert!(!is_shavian("shavian"));
        assert!(!is_shavian("𐑣𐑲 𐑞𐑺"));
    }
}
