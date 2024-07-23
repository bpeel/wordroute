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

mod grid;
mod build;
mod dictionary;
mod directions;
mod word_finder;
mod counts;

use std::path::Path;
use std::io::{BufReader, BufRead};
use std::{fs, process::ExitCode, ffi::OsString};
use clap::Parser;
use std::collections::HashSet;

#[derive(PartialEq, Eq, Clone, Copy)]
enum WordType {
    Normal,
    Bonus,
    Excluded,
}

#[derive(Parser)]
#[command(name = "Build")]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    dictionary: OsString,
    #[arg(short, long, value_name = "FILE")]
    bonus_words: Vec<OsString>,
    #[arg(short = 'x', long, value_name = "FILE")]
    excluded_words: Vec<OsString>,
    #[arg(short, long, value_name = "LENGTH", default_value_t = 4)]
    minimum_length: usize,
    #[arg(short = 'H', long)]
    human_readable: bool,
}

fn print_grid(grid: &grid::Grid, counts: &counts::GridCounts) {
    for y in 0..grid.height() {
        if y & 1 != 0 {
            print!("   ");
        }

        for x in 0..grid.width() {
            print!("  {}   ", grid.at(x, y));
        }

        println!();

        if y & 1 != 0 {
            print!("   ");
        }

        for x in 0..grid.width() {
            let counts = counts.at(x, y);
            print!("{:>2} {:<3}", counts.starts, counts.visits);
        }

        println!();
    }
}

fn print_human_readable(
    grid: &grid::Grid,
    counts: &counts::GridCounts,
    words: Vec<(String, WordType)>,
) {
    print_grid(&grid, &counts);

    println!();

    for (word, word_type) in words.into_iter() {
        print!("{}", &word);

        match word_type {
            WordType::Normal => (),
            WordType::Bonus => {
                print!(" (bonus)");
            },
            WordType::Excluded => {
                print!(" (excluded)");
            },
        }

        println!();
    }
}

fn print_json(
    grid: &grid::Grid,
    words: Vec<(String, WordType)>,
) {
    print!("{{\"grid\":\"");

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            print!("{}", grid.at(x, y));
        }
        if y < grid.height() - 1 {
            print!("\\n");
        }
    }

    print!("\",\"words\":{{");

    for (i, (word, word_type)) in words.into_iter().enumerate() {
        if i != 0 {
            print!(",");
        }

        let word_type = match word_type {
            WordType::Normal => 0,
            WordType::Bonus => 1,
            WordType::Excluded => 2,
        };

        print!("\"{}\":{}", word, word_type);
    }

    println!("}}}}");
}

fn read_word_list_from_file<P: AsRef<Path>>(
    filename: P,
    words: &mut HashSet<String>,
) -> Result<(), std::io::Error> {
    for line in BufReader::new(std::fs::File::open(filename)?).lines() {
        let line = line?;
        let line = line.trim();

        if !line.is_empty() && !line.starts_with('#') {
            words.insert(line.to_string());
        }
    }

    Ok(())
}

fn read_word_list<I, P>(
    filenames: I,
) -> Result<HashSet<String>, std::io::Error>
    where I: IntoIterator<Item = P>,
          P: AsRef<Path>,
{
    let mut words = HashSet::new();

    for filename in filenames {
        read_word_list_from_file(&filename, &mut words)
            .map_err(|e| {
                let kind = e.kind();
                std::io::Error::new(
                    kind,
                    format!(
                        "{}: {}",
                        filename.as_ref().to_string_lossy(),
                        e,
                    ))
            })?;
    }

    Ok(words)
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let dictionary = match fs::read(&cli.dictionary) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", cli.dictionary.to_string_lossy(), e);
            return ExitCode::FAILURE;
        },
    };

    let bonus_words = match read_word_list(cli.bonus_words.iter()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };

    let excluded_words = match read_word_list(cli.excluded_words.iter()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };

    let dictionary = dictionary::Dictionary::new(dictionary.into_boxed_slice());

    let grid_string = match std::io::read_to_string(std::io::stdin()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("stdin: {}", e);
            return ExitCode::FAILURE;
        },
    };

    let grid = match grid::Grid::new(&grid_string) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("stdin: {}", e);
            return ExitCode::FAILURE;
        },
    };

    let words = build::search_words(&grid, &dictionary, cli.minimum_length);
    let mut words = words.into_iter()
        .map(|word| {
            let word_type = if excluded_words.contains(&word) {
                WordType::Excluded
            } else if bonus_words.contains(&word) {
                WordType::Bonus
            } else {
                WordType::Normal
            };
            (word, word_type)
        })
        .collect::<Vec<(String, WordType)>>();

    words.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    if cli.human_readable {
        let counts = build::count_visits(
            &grid,
            words.iter().filter_map(|&(ref word, word_type)| {
                (word_type == WordType::Normal).then_some(word)
            })
        );

        print_human_readable(&grid, &counts, words);
    } else {
        print_json(&grid, words);
    }

    ExitCode::SUCCESS
}
