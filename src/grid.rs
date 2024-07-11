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

#[derive(Debug)]
pub struct Grid {
    values: Box<[char]>,
    width: u32,
    height: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyGrid
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::EmptyGrid => write!(f, "empty grid"),
        }
    }
}

impl Grid {
    pub fn new(s: &str) -> Result<Grid, Error> {
        // Find the longest line
        let width = s.lines().map(|line| line.trim_end().len())
            .max()
            .unwrap_or(0);

        if width < 1 {
            return Err(Error::EmptyGrid);
        }

        let mut values = Vec::new();

        for (row, line) in s.lines().enumerate() {
            let line = line.trim_end();

            if !line.is_empty() {
                values.resize(row * width, ' ');
            }

            values.extend(line.chars());
        }

        let height = (values.len() + width - 1) / width;

        values.resize(width * height, ' ');

        Ok(Grid {
            values: values.into_boxed_slice(),
            width: width as u32,
            height: height as u32,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn at(&self, x: u32, y: u32) -> char {
        assert!(x < self.width);

        self.values[(y * self.width + x) as usize]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_grid() {
        assert_eq!(Grid::new("").unwrap_err(), Error::EmptyGrid);
        assert_eq!(Grid::new("   ").unwrap_err(), Error::EmptyGrid);
        assert_eq!(Grid::new(" \n  ").unwrap_err(), Error::EmptyGrid);
        assert_eq!(&Grid::new("").unwrap_err().to_string(), "empty grid");
    }

    #[test]
    fn trailing_empty_lines() {
        let grid = Grid::new("a\nb\n").unwrap();

        assert_eq!(grid.width(), 1);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.at(0, 0), 'a');
        assert_eq!(grid.at(0, 1), 'b');

        let grid = Grid::new("a\nb\n     \n     ").unwrap();

        assert_eq!(grid.width(), 1);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.at(0, 0), 'a');
        assert_eq!(grid.at(0, 1), 'b');
    }

    #[test]
    fn short_lines() {
        let grid = Grid::new("ab\nc\nd       ").unwrap();

        assert_eq!(grid.width(), 2);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid.at(0, 0), 'a');
        assert_eq!(grid.at(1, 0), 'b');
        assert_eq!(grid.at(0, 1), 'c');
        assert_eq!(grid.at(1, 1), ' ');
        assert_eq!(grid.at(0, 2), 'd');
        assert_eq!(grid.at(1, 2), ' ');
    }
}
