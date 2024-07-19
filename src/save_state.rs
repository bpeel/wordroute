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

#[derive(Debug)]
pub struct SaveState {
    misses: u32,
    hints_used: bool,
    found_words: Vec<u32>,
}

impl SaveState {
    pub fn new<I>(
        misses: u32,
        hints_used: bool,
        found_words: I,
    ) -> SaveState
        where I: IntoIterator<Item = usize>
    {
        let mut found_words_vec = Vec::new();

        for word in found_words {
            let pos = word / u32::BITS as usize;

            if pos + 1 > found_words_vec.len() {
                found_words_vec.resize(pos + 1, 0);
            }

            found_words_vec[pos] |= 1 << (word % u32::BITS as usize);
        }

        SaveState {
            misses,
            hints_used,
            found_words: found_words_vec,
        }
    }

    pub fn misses(&self) -> u32 {
        return self.misses;
    }

    pub fn hints_used(&self) -> bool {
        return self.hints_used;
    }

    pub fn found_words(&self) -> FoundWords {
        return FoundWords::new(&self.found_words)
    }
}

pub struct FoundWords<'a> {
    bits: u32,
    pos: usize,
    slice: &'a [u32],
}

impl<'a> FoundWords<'a> {
    fn new(slice: &'a [u32]) -> FoundWords {
        let &bits = slice.get(0).unwrap_or(&0);

        FoundWords {
            bits,
            pos: 0,
            slice,
        }
    }
}

impl<'a> Iterator for FoundWords<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        loop {
            if self.bits != 0 {
                let next_bit = self.bits.trailing_zeros();
                self.bits &= u32::MAX.wrapping_shl(next_bit).wrapping_shl(1);
                break Some(next_bit as usize + self.pos * u32::BITS as usize);
            } else if let Some(&bits) = self.slice.get(self.pos + 1) {
                self.bits = bits;
                self.pos += 1;
            } else {
                break None;
            }
        }
    }

    fn count(self) -> usize {
        self.slice.get(self.pos + 1..)
            .map(|slice| {
                slice.into_iter().map(|bits| {
                    bits.count_ones() as usize
                }).sum()
            })
            .unwrap_or(0) +
            self.bits.count_ones() as usize
    }
}

impl fmt::Display for SaveState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}.{}.", self.misses, self.hints_used as u8)?;

        if let Some(last_part) =
            self.found_words.iter().rposition(|&p| p != 0)
        {
            for &part in self.found_words[0..last_part].iter() {
                write!(f, "{:08x}", part)?;
            }
            write!(f, "{:x}", self.found_words[last_part])?;
        } else {
            write!(f, "0")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidMisses,
    InvalidHintsUsed,
    InvalidFoundWords,
    TrailingText,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            Error::InvalidMisses => "invalid misses",
            Error::InvalidHintsUsed => "invalid hints used",
            Error::InvalidFoundWords => "invalid found words",
            Error::TrailingText => "trailing text",
        };

        write!(f, "{}", text)
    }
}

fn parse_found_words(mut s: &str) -> Option<Vec<u32>> {
    let mut found_words = Vec::new();

    while s.len() > 8 {
        found_words.push(u32::from_str_radix(s.get(0..8)?, 16).ok()?);
        s = &s[8..];
    }

    found_words.push(u32::from_str_radix(s, 16).ok()?);

    Some(found_words)
}

impl FromStr for SaveState {
    type Err = Error;

    fn from_str(s: &str) -> Result<SaveState, Error> {
        let mut parts = s.split('.');

        let Some(misses) = parts.next()
            .and_then(|p| u32::from_str_radix(p, 16).ok())
        else {
            return Err(Error::InvalidMisses);
        };

        let hints_used = match parts.next() {
            Some("1") => true,
            Some("0") => false,
            _ => return Err(Error::InvalidHintsUsed),
        };

        let Some(found_words) = parts.next().and_then(|p| parse_found_words(p))
        else {
            return Err(Error::InvalidFoundWords);
        };

        if parts.next().is_some() {
            return Err(Error::TrailingText);
        }

        Ok(SaveState {
            misses,
            hints_used,
            found_words,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn found_words() {
        let bits = [0, 0, 2];
        let mut words = FoundWords::new(&bits);
        assert_eq!(words.next().unwrap(), 65);
        assert!(words.next().is_none());

        let bits = [u32::MAX, u32::MAX, 2, 0, 0];
        let mut words = FoundWords::new(&bits);

        for i in 0..64 {
            assert_eq!(words.next().unwrap(), i);
        }

        assert_eq!(words.next().unwrap(), 65);
        assert!(words.next().is_none());
    }

    #[test]
    fn display() {
        assert_eq!(
            &SaveState::new(255, true, vec![0, 31, 32, 128]).to_string(),
            "ff.1.800000010000000100000000000000001",
        );
        assert_eq!(
            &SaveState::new(0, false, vec![]).to_string(),
            "0.0.0",
        );
    }

    #[test]
    fn test_parse_found_words() {
        assert_eq!(&parse_found_words("ffffffff").unwrap(), &[0xffffffff]);
        assert_eq!(
            &parse_found_words("123456789").unwrap(),
            &[0x12345678, 0x9],
        );
    }

    #[test]
    fn parse() {
        let save_state = "ff.1.800000010000000100000000000000001"
            .parse::<SaveState>().unwrap();
        assert_eq!(save_state.misses(), 255);
        assert!(save_state.hints_used());
        assert_eq!(
            save_state.found_words().collect::<Vec<_>>(),
            [0, 31, 32, 128],
        );

        let save_state = "0.0.0".parse::<SaveState>().unwrap();
        assert_eq!(save_state.misses(), 0);
        assert!(!save_state.hints_used());
        assert!(save_state.found_words().next().is_none());
    }

    #[test]
    fn parse_error() {
        assert_eq!(
            &"".parse::<SaveState>().unwrap_err().to_string(),
            "invalid misses",
        );
        assert_eq!(
            &"g.0.0".parse::<SaveState>().unwrap_err().to_string(),
            "invalid misses",
        );
        assert_eq!(
            &"0.".parse::<SaveState>().unwrap_err().to_string(),
            "invalid hints used",
        );
        assert_eq!(
            &"0.2.0".parse::<SaveState>().unwrap_err().to_string(),
            "invalid hints used",
        );
        assert_eq!(
            &"0.1.".parse::<SaveState>().unwrap_err().to_string(),
            "invalid found words",
        );
        assert_eq!(
            &"0.1.g".parse::<SaveState>().unwrap_err().to_string(),
            "invalid found words",
        );
    }

    #[test]
    fn count() {
        assert_eq!(
            "0.0.800000001".parse::<SaveState>().unwrap().found_words().count(),
            2,
        );
        assert_eq!(
            "0.0.0".parse::<SaveState>().unwrap().found_words().count(),
            0,
        );

        let save_state = "0.0.80000001000000011".parse::<SaveState>().unwrap();

        for i in 0..=4 {
            assert_eq!(
                save_state.found_words().skip(i).count(),
                4 - i,
            );
        }
    }
}
