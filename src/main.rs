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

use std::{fs, process::ExitCode, ffi::OsString};
use clap::Parser;

#[derive(Parser)]
#[command(name = "Build")]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    dictionary: OsString,
    #[arg(short, long, value_name = "LENGTH", default_value_t = 4)]
    minimum_length: usize,
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

    for word in words.into_iter() {
        println!("{}", word);
    }

    ExitCode::SUCCESS
}
