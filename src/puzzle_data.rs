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

use std::fmt;
use std::str::FromStr;
use super::grid::{self, Grid};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum WordType {
    Normal,
    Bonus,
    Excluded,
}

#[derive(Debug)]
pub struct PuzzleData {
    pub grid: Grid,
    pub words: Vec<(String, WordType)>,
}

#[derive(Debug)]
pub enum Error {
    GridError(grid::Error),
    EmptyWord,
    InvalidWordType,
}

impl From<grid::Error> for Error {
    fn from(e: grid::Error) -> Error {
        Error::GridError(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::GridError(e) => e.fmt(f),
            Error::InvalidWordType => write!(f, "invalid word type"),
            Error::EmptyWord => write!(f, "empty word"),
        }
    }
}

impl FromStr for PuzzleData {
    type Err = Error;

    fn from_str(s: &str) -> Result<PuzzleData, Error> {
        let mut words = Vec::new();

        let grid = match s.split_once(',') {
            None => Grid::new(s)?,
            Some((grid_str, tail)) => {
                for part in tail.split(',') {
                    words.push(parse_word(part)?);
                }

                Grid::new(grid_str)?
            },
        };

        Ok(PuzzleData { grid, words })
    }
}

impl fmt::Display for PuzzleData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.grid.fmt(f)?;

        for (word, word_type) in self.words.iter() {
            write!(f, ",{}", word)?;

            match word_type {
                WordType::Normal => (),
                WordType::Bonus => write!(f, ":b")?,
                WordType::Excluded => write!(f, ":x")?,
            }
        }

        Ok(())
    }
}

fn parse_word(s: &str) -> Result<(String, WordType), Error> {
    let (word, word_type) = match s.split_once(':') {
        None => (s.to_string(), WordType::Normal),
        Some((word, word_type)) => {
            let word_type = match word_type {
                "b" => WordType::Bonus,
                "x" => WordType::Excluded,
                _ => return Err(Error::InvalidWordType),
            };

            (word.to_string(), word_type)
        }
    };

    if word.is_empty() {
        Err(Error::EmptyWord)
    } else {
        Ok((word, word_type))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let puzzle = "𐑖".parse::<PuzzleData>().unwrap();

        assert_eq!(puzzle.grid.width(), 1);
        assert_eq!(puzzle.grid.height(), 1);
        assert_eq!(puzzle.grid.at(0, 0), '𐑖');
        assert!(&puzzle.words.is_empty());

        let puzzle = "AB:CB,arm,noggin:b,bum:x".parse::<PuzzleData>().unwrap();
        assert_eq!(puzzle.grid.width(), 2);
        assert_eq!(puzzle.grid.height(), 2);
        assert_eq!(puzzle.grid.at(1, 1), '𐑑');
        assert_eq!(
            &puzzle.words.iter()
                .map(|(w, t)| (w.as_str(), *t))
                .collect::<Vec<(&str, _)>>(),
            &[
                ("arm", WordType::Normal),
                ("noggin", WordType::Bonus),
                ("bum", WordType::Excluded),
            ],
        );
    }

    #[test]
    fn parse_error() {
        assert_eq!(
            &"".parse::<PuzzleData>().unwrap_err().to_string(),
            "empty grid",
        );
        assert_eq!(
            &"a,aĉa:e".parse::<PuzzleData>().unwrap_err().to_string(),
            "invalid word type",
        );
        assert_eq!(
            &"a,".parse::<PuzzleData>().unwrap_err().to_string(),
            "empty word",
        );
        assert_eq!(
            &"a,:b".parse::<PuzzleData>().unwrap_err().to_string(),
            "empty word",
        );
    }

    #[test]
    fn display() {
        assert_eq!(&"a".parse::<PuzzleData>().unwrap().to_string(), "a");
        assert_eq!(
            &"a,head,noggin:b,bum:x".parse::<PuzzleData>().unwrap().to_string(),
            "a,head,noggin:b,bum:x",
        );
    }
}
