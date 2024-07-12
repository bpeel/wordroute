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

use std::{fs, process::ExitCode, ffi::OsString};
use clap::Parser;

#[derive(Parser)]
#[command(name = "Build")]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    dictionary: OsString,
    #[arg(short, long, value_name = "LENGTH", default_value_t = 4)]
    minimum_length: usize,
    #[arg(short, long)]
    text: bool,
}

fn print_grid(grid: &grid::Grid, counts: &counts::GridCounts) {
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            print!("  {}   ", grid.at(x, y));
        }

        println!();

        for x in 0..grid.width() {
            let counts = counts.at(x, y);
            print!("{:>2} {:<3}", counts.starts, counts.visits);
        }

        println!();
    }
}

fn print_text(
    grid: &grid::Grid,
    counts: &counts::GridCounts,
    words: Vec<String>,
) {
    print_grid(&grid, &counts);

    println!();

    for word in words.into_iter() {
        println!("{}", word);
    }
}

fn print_json(
    grid: &grid::Grid,
    counts: &counts::GridCounts,
    words: Vec<String>,
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

    print!("\",\"counts\":[");

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if x != 0 || y != 0 {
                print!(",");
            }
            let count = counts.at(x, y);
            print!("{},{}", count.starts, count.visits);
        }
    }

    print!("],\"words\":{{");

    for (i, word) in words.into_iter().enumerate() {
        if i != 0 {
            print!(",");
        }

        let word_type = 0;

        print!("\"{}\":{}", word, word_type);
    }

    println!("}}}}");
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
    let mut words = words.into_iter().collect::<Vec<_>>();
    words.sort();
    let counts = build::count_visits(&grid, words.iter());

    if cli.text {
        print_text(&grid, &counts, words);
    } else {
        print_json(&grid, &counts, words);
    }

    ExitCode::SUCCESS
}