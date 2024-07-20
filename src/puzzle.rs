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

use super::grid::Grid;
use super::counts::GridCounts;
use super::word_finder;
use super::directions;
use std::collections::{hash_map, HashMap, HashSet};
use std::fmt::Write;

pub const MIN_WORD_LENGTH: usize = 4;
pub const N_HINT_LEVELS: usize = 4;

macro_rules! show_word_message {
    ( $puzzle:expr, $format:literal, $( $x:expr ),* ) => {
        {
            $puzzle.pending_word_message.clear();
            write!(
                &mut $puzzle.pending_word_message,
                $format,
                $( $x, )*
            ).unwrap();
            $puzzle.has_pending_word_message = true;
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum WordType {
    Normal,
    Bonus,
}

pub struct Word {
    pub word_type: WordType,
    pub length: usize,
    pub found: bool,
}

pub struct Puzzle {
    grid: Grid,
    counts: GridCounts,
    words: HashMap<String, Word>,
    word_finder: word_finder::Finder,
    route_buf: Vec<u8>,
    n_words_found: usize,
    total_n_words: usize,
    n_letters_found: usize,
    total_n_letters: usize,
    hint_level: usize,
    misses: u32,
    hints_used: bool,

    has_pending_word_message: bool,
    pending_word_message: String,

    counts_dirty: u64,
    n_words_found_dirty: bool,
    n_letters_found_dirty: bool,
    word_lists_dirty: u64,
    hint_level_dirty: bool,
}

impl Puzzle {
    pub fn new<I>(
        grid: Grid,
        counts: GridCounts,
        words: I,
    ) -> Puzzle
        where I: IntoIterator<Item = (String, WordType)>
    {
        let words = words.into_iter()
            .map(|(key, word_type)| {
                let word = Word {
                    word_type,
                    length: key.chars().count(),
                    found: false,
                };
                (key, word)
            })
            .collect::<HashMap<_, _>>();

        let total_n_words = words.values().filter(|w| {
            w.word_type == WordType::Normal
        }).count();

        let total_n_letters = words.values().filter_map(|w| {
            if w.word_type == WordType::Normal {
                Some(w.length)
            } else {
                None
            }
        }).sum::<usize>();

        let counts_dirty = u64::MAX >>
            (u64::BITS - grid.width() * grid.height());

        let mut word_lists_dirty = 0;

        for word in words.values() {
            if word.word_type == WordType::Normal {
                word_lists_dirty |= 1 << word.length;
            }
        }

        Puzzle {
            grid,
            counts,
            words,
            word_finder: word_finder::Finder::new(),
            route_buf: Vec::new(),
            n_words_found: 0,
            total_n_words,
            n_letters_found: 0,
            total_n_letters,
            hint_level: 0,
            misses: 0,
            hints_used: false,

            has_pending_word_message: false,
            pending_word_message: String::new(),

            counts_dirty,
            n_words_found_dirty: true,
            n_letters_found_dirty: true,
            word_lists_dirty,
            hint_level_dirty: true,
        }
    }

    fn show_word_message(&mut self, message: &str) {
        self.pending_word_message.clear();
        self.pending_word_message.push_str(message);
        self.has_pending_word_message = true;
    }

    pub fn score_word(&mut self, word: &str) {
        let length = word.chars().count();

        if length < MIN_WORD_LENGTH {
            if length > 0 {
                self.show_word_message("Too short");
            }
        } else if let Some(word_data) = self.words.get_mut(word) {
            if std::mem::replace(&mut word_data.found, true) {
                match word_data.word_type {
                    WordType::Bonus => {
                        self.show_word_message("Already found (bonus)");
                    },
                    WordType::Normal => {
                        self.show_word_message("Already found");
                    }
                }
            } else {
                match word_data.word_type {
                    WordType::Bonus => self.show_word_message("Bonus word!"),
                    WordType::Normal => {
                        show_word_message!(self, "+{} points!", length);
                        self.remove_visits_for_word(word);
                        self.n_words_found += 1;
                        self.n_words_found_dirty = true;
                        self.n_letters_found += length;
                        self.n_letters_found_dirty = true;
                        self.update_hint_level();
                        self.word_lists_dirty |= 1 << length;
                    }
                }
            }
        } else {
            self.show_word_message("Not in list");
            self.misses += 1;
        }
    }

    fn update_hint_level(&mut self) {
        let new_hint_level = self.n_letters_found *
            N_HINT_LEVELS /
            self.total_n_letters;

        if new_hint_level != self.hint_level {
            self.hint_level = new_hint_level;
            self.hint_level_dirty = true;
        }
    }

    fn dirty_counts_at_pos(&mut self, x: u32, y: u32) {
        self.counts_dirty |= 1 << (y * self.grid.width() + x);
    }

    fn remove_visits_for_word(&mut self, word: &str) {
        let mut route_buf = std::mem::take(&mut self.route_buf);

        route_buf.clear();

        if let Some((mut x, mut y)) = self.word_finder.find(
            &self.grid,
            &word,
            &mut route_buf,
        ) {
            let start = self.counts.at_mut(x, y);
            start.starts -= 1;
            start.visits -= 1;
            self.dirty_counts_at_pos(x, y);

            for &dir in route_buf.iter() {
                (x, y) = directions::step(x, y, dir);

                self.counts.at_mut(x, y).visits -= 1;

                self.dirty_counts_at_pos(x, y);
            }
        }

        self.route_buf = route_buf;
    }

    pub fn pending_word_message(&mut self) -> Option<&str> {
        if self.has_pending_word_message {
            self.has_pending_word_message = false;
            Some(&self.pending_word_message)
        } else {
            None
        }
    }

    pub fn changed_counts(&mut self) -> ChangedCounts {
        ChangedCounts::new(
            self.grid.width(),
            std::mem::take(&mut self.counts_dirty),
        )
    }

    pub fn changed_n_words_found(&mut self) -> Option<usize> {
        if self.n_words_found_dirty {
            self.n_words_found_dirty = false;
            Some(self.n_words_found)
        } else {
            None
        }
    }

    pub fn changed_n_letters_found(&mut self) -> Option<usize> {
        if self.n_letters_found_dirty {
            self.n_letters_found_dirty = false;
            Some(self.n_letters_found)
        } else {
            None
        }
    }

    pub fn changed_hint_level(&mut self) -> Option<usize> {
        if self.hint_level_dirty {
            self.hint_level_dirty = false;
            Some(self.hint_level)
        } else {
            None
        }
    }

    pub fn changed_word_lists(&mut self) -> ChangedWordLists {
        ChangedWordLists::new(std::mem::take(&mut self.word_lists_dirty))
    }

    pub fn total_n_words(&self) -> usize {
        self.total_n_words
    }

    pub fn total_n_letters(&self) -> usize {
        self.total_n_letters
    }

    pub fn use_hints(&mut self) {
        self.hints_used = true;
    }

    pub fn width(&self) -> u32 {
        self.grid.width()
    }

    pub fn height(&self) -> u32 {
        self.grid.height()
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn counts(&self) -> &GridCounts {
        &self.counts
    }

    pub fn word_lists(&self) -> Vec<usize> {
        let mut lengths = self.words.values()
            .map(|word| word.length)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        lengths.sort_unstable();

        lengths
    }

    pub fn words(&self) -> Words {
        Words::new(self.words.iter())
    }
}

pub struct ChangedCounts {
    grid_width: u32,
    counts_dirty: u64,
}

impl ChangedCounts {
    fn new(grid_width: u32, counts_dirty: u64) -> ChangedCounts {
        ChangedCounts {
            grid_width,
            counts_dirty,
        }
    }
}

impl Iterator for ChangedCounts {
    type Item = (u32, u32);

    fn next(&mut self) -> Option<(u32, u32)> {
        if self.counts_dirty == 0 {
            None
        } else {
            let index = self.counts_dirty.trailing_zeros();
            self.counts_dirty &= u64::MAX.wrapping_shl(index).wrapping_shl(1);
            Some((
                index as u32 % self.grid_width,
                index as u32 / self.grid_width
            ))
        }
    }
}

pub struct ChangedWordLists {
    lists_dirty: u64,
}

impl ChangedWordLists {
    fn new(lists_dirty: u64) -> ChangedWordLists {
        ChangedWordLists {
            lists_dirty,
        }
    }
}

impl Iterator for ChangedWordLists {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.lists_dirty == 0 {
            None
        } else {
            let index = self.lists_dirty.trailing_zeros();
            self.lists_dirty &= u64::MAX.wrapping_shl(index).wrapping_shl(1);
            Some(index as usize)
        }
    }
}

pub struct Words<'a> {
    inner: hash_map::Iter<'a, String, Word>,
}

impl<'a> Iterator for Words<'a> {
    type Item = (&'a str, &'a Word);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(key, word)| (key.as_str(), word))
    }
}

impl<'a> Words<'a> {
    fn new(inner: hash_map::Iter<'a, String, Word>) -> Words {
        Words {
            inner
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn four_line_puzzle() -> Puzzle {
        let grid = Grid::new(
            "potatostompwhips\n\
             abcdefghijklmnop\n\
             xxxxxxxxxxxxxxxx\n\
             yyyyyyyyyyyyyyyy"
        ).unwrap();

        let counts_data = vec![
            (2, 2), (0, 2), (0, 2), (0, 2), (0, 2), (0, 2), (1, 2), (0, 2),
            (0, 2), (0, 2), (0, 2), (1, 2), (0, 2), (0, 2), (0, 2), (0, 2),
            (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1),
            (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1),
            (1, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1),
            (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1),
            (1, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1),
            (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1)
        ];

        let mut counts = GridCounts::new(grid.width(), grid.height());

        for (i, (starts, visits)) in counts_data.into_iter().enumerate() {
            let counts = counts.at_mut(i as u32 % 16, i as u32 / 16);
            counts.starts = starts;
            counts.visits = visits;
        }

        Puzzle::new(
            grid,
            counts,
            vec![
                ("potato".to_string(), WordType::Normal),
                ("stomp".to_string(), WordType::Normal),
                ("whips".to_string(), WordType::Normal),
                (
                    "paobtcadteofsgthoimjpkwlhminposp".to_string(),
                    WordType::Bonus,
                ),
            ]
        )
    }

    #[test]
    fn score_word() {
        let mut puzzle = four_line_puzzle();

        assert_eq!(puzzle.total_n_words(), 3);
        assert_eq!(puzzle.total_n_letters(), 16);
        assert_eq!(puzzle.width(), puzzle.grid().width());
        assert_eq!(puzzle.height(), puzzle.grid().height());
        assert_eq!(puzzle.words().count(), 4);

        assert!(puzzle.pending_word_message().is_none());

        assert_eq!(
            &puzzle.changed_counts().collect::<Vec<_>>(),
            &(0..4).map(|y| (0..16).map(move |x| (x, y))).flatten()
                .collect::<Vec<_>>(),
        );

        assert!(puzzle.changed_counts().next().is_none());

        assert_eq!(puzzle.changed_n_words_found().unwrap(), 0);
        assert!(puzzle.changed_n_words_found().is_none());

        assert_eq!(puzzle.changed_n_letters_found().unwrap(), 0);
        assert!(puzzle.changed_n_letters_found().is_none());

        assert_eq!(puzzle.changed_hint_level().unwrap(), 0);
        assert!(puzzle.changed_hint_level().is_none());

        assert_eq!(
            &puzzle.changed_word_lists().collect::<Vec<_>>(),
            &[5, 6],
        );
        assert!(puzzle.changed_word_lists().next().is_none());

        assert_eq!(&puzzle.word_lists(), &[5, 6, 32]);

        puzzle.score_word("potato");

        assert_eq!(puzzle.changed_hint_level().unwrap(), 1);
        assert!(puzzle.changed_hint_level().is_none());

        puzzle.score_word("stomp");

        assert_eq!(puzzle.changed_hint_level().unwrap(), 2);
        assert!(puzzle.changed_hint_level().is_none());

        assert_eq!(puzzle.pending_word_message().unwrap(), "+5 points!");
        assert!(puzzle.pending_word_message().is_none());

        puzzle.score_word("paobtcadteofsgthoimjpkwlhminposp");
        assert_eq!(puzzle.pending_word_message().unwrap(), "Bonus word!");
        assert!(puzzle.pending_word_message().is_none());

        assert_eq!(puzzle.changed_n_words_found().unwrap(), 2);
        assert!(puzzle.changed_n_words_found().is_none());

        assert_eq!(puzzle.changed_n_letters_found().unwrap(), 11);
        assert!(puzzle.changed_n_letters_found().is_none());

        assert_eq!(
            &puzzle.changed_counts().collect::<Vec<_>>(),
            &[
                (0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0),
                (6, 0), (7, 0), (8, 0), (9, 0), (10, 0),
            ],
        );

        assert_eq!(puzzle.counts().at(0, 0).starts, 1);
        assert_eq!(puzzle.counts().at(0, 0).visits, 1);
        assert_eq!(puzzle.counts().at(0, 1).starts, 0);
        assert_eq!(puzzle.counts().at(0, 1).visits, 1);

        assert!(puzzle.changed_counts().next().is_none());

        assert_eq!(
            &puzzle.changed_word_lists().collect::<Vec<_>>(),
            &[5, 6],
        );
        assert!(puzzle.changed_word_lists().next().is_none());
    }
}
