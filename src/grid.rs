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
use super::shavicode;

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

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..self.height() {
            if y > 0 {
                write!(f, ":")?;
            }

            for x in 0..self.width() {
                write!(f, "{}", shavicode::encode_char(self.at(x, y)))?;
            }
        }

        Ok(())
    }
}

fn lines(s: &str) -> std::str::Split<&[char]> {
    s.split(&['\n', ':'])
}

impl Grid {
    pub fn new(s: &str) -> Result<Grid, Error> {
        // Find the longest line
        let width = lines(s).map(|line| {
            line.chars().filter(|ch| !ch.is_whitespace()).count()
        }).max().unwrap_or(0);

        if width < 1 {
            return Err(Error::EmptyGrid);
        }

        let mut values = Vec::new();

        for (row, line) in lines(s).enumerate() {
            let line = line.trim_end();

            if !line.is_empty() {
                values.resize(row * width, '.');
                values.extend(
                    line.chars()
                        .filter_map(|ch| {
                            (!ch.is_whitespace()).then(|| {
                                shavicode::decode_char(ch)
                            })
                        })
                );
            }
        }

        let height = (values.len() + width - 1) / width;

        values.resize(width * height, '.');

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
        assert_eq!(grid.at(0, 0), '𐑪');
        assert_eq!(grid.at(0, 1), '𐑫');

        let grid = Grid::new("a\nb\n     \n     ").unwrap();

        assert_eq!(grid.width(), 1);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.at(0, 0), '𐑪');
        assert_eq!(grid.at(0, 1), '𐑫');
    }

    #[test]
    fn short_lines() {
        let grid = Grid::new("AB\nC\nD       ").unwrap();

        assert_eq!(grid.width(), 2);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid.at(0, 0), '𐑐');
        assert_eq!(grid.at(1, 0), '𐑑');
        assert_eq!(grid.at(0, 1), '𐑒');
        assert_eq!(grid.at(1, 1), '.');
        assert_eq!(grid.at(0, 2), '𐑓');
        assert_eq!(grid.at(1, 2), '.');
    }

    #[test]
    fn multibyte() {
        let grid = Grid::new(
            "𐑖𐑷𐑦𐑟\n\
             𐑜𐑮𐑱𐑑",
        ).unwrap();

        assert_eq!(grid.width(), 4);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.at(0, 0), '𐑖');
        assert_eq!(grid.at(1, 0), '𐑷');
        assert_eq!(grid.at(2, 0), '𐑦');
        assert_eq!(grid.at(3, 0), '𐑟');
        assert_eq!(grid.at(0, 1), '𐑜');
        assert_eq!(grid.at(1, 1), '𐑮');
        assert_eq!(grid.at(2, 1), '𐑱');
        assert_eq!(grid.at(3, 1), '𐑑');
    }

    #[test]
    fn ignore_spaces() {
        let grid = Grid::new(
            "  a     b     c\n\
             d  e\tf",
        ).unwrap();

        assert_eq!(grid.width(), 3);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.at(0, 0), '𐑪');
        assert_eq!(grid.at(1, 0), '𐑫');
        assert_eq!(grid.at(2, 0), '𐑬');
        assert_eq!(grid.at(0, 1), '𐑭');
        assert_eq!(grid.at(1, 1), '𐑮');
        assert_eq!(grid.at(2, 1), '𐑯');
    }

    #[test]
    fn colons() {
        let grid = Grid::new(
            "  a b c   :  d ef"
        ).unwrap();

        assert_eq!(grid.width(), 3);
        assert_eq!(grid.height(), 2);
        assert_eq!(grid.at(0, 0), '𐑪');
        assert_eq!(grid.at(1, 0), '𐑫');
        assert_eq!(grid.at(2, 0), '𐑬');
        assert_eq!(grid.at(0, 1), '𐑭');
        assert_eq!(grid.at(1, 1), '𐑮');
        assert_eq!(grid.at(2, 1), '𐑯');
    }

    #[test]
    fn format() {
        assert_eq!(&Grid::new("a").unwrap().to_string(), "a");
        assert_eq!(&Grid::new("abc\ndef").unwrap().to_string(), "abc:def");
    }
}
