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
use super::save_state::SaveState;
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
    Excluded,
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

    pending_excluded_word: bool,
    pending_finish: bool,

    counts_dirty: u64,
    n_words_found_dirty: bool,
    n_letters_found_dirty: bool,
    word_lists_dirty: u64,
    hint_level_dirty: bool,
    save_state_dirty: bool,
}

impl Puzzle {
    pub fn new<I>(
        grid: Grid,
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
            (w.word_type == WordType::Normal).then_some(w.length)
        }).sum::<usize>();

        let counts_dirty = u64::MAX >>
            (u64::BITS - grid.width() * grid.height());

        let mut word_lists_dirty = 0;

        for word in words.values() {
            if word.word_type == WordType::Normal {
                word_lists_dirty |= 1 << word.length;
            }
        }

        let mut word_finder = word_finder::Finder::new();
        let mut route_buf = Vec::new();

        let counts = generate_counts(
            &grid,
            &mut word_finder,
            &mut route_buf,
            words.iter().filter_map(|(key, word)| {
                (word.word_type == WordType::Normal).then_some(key)
            }),
        );

        Puzzle {
            grid,
            counts,
            words,
            word_finder,
            route_buf,
            n_words_found: 0,
            total_n_words,
            n_letters_found: 0,
            total_n_letters,
            hint_level: 0,
            misses: 0,
            hints_used: false,

            has_pending_word_message: false,
            pending_word_message: String::new(),

            pending_excluded_word: false,
            pending_finish: false,

            counts_dirty,
            n_words_found_dirty: true,
            n_letters_found_dirty: true,
            word_lists_dirty,
            hint_level_dirty: true,
            save_state_dirty: false,
        }
    }

    pub fn load_save_state(&mut self, save_state: &SaveState) {
        if self.misses < save_state.misses() {
            self.misses = save_state.misses();
        }

        if save_state.hints_used() {
            self.hints_used = true;
        }

        let mut sorted_words = self.words.iter_mut().collect::<Vec<_>>();
        sorted_words.sort_unstable_by_key(|&(word, _)| word);

        let mut words_to_score = Vec::new();

        for word_num in save_state.found_words() {
            if let Some((word, word_data)) = sorted_words.get_mut(word_num) {
                if !word_data.found {
                    word_data.found = true;
                    if word_data.word_type == WordType::Normal {
                        words_to_score.push(
                            (word.to_string(), word_data.length)
                        );
                    }
                }
            }
        }

        for (word, length) in words_to_score.into_iter() {
            self.score_normal_word(&word, length);
        }

        self.save_state_dirty = false;
    }

    fn show_word_message(&mut self, message: &str) {
        self.pending_word_message.clear();
        self.pending_word_message.push_str(message);
        self.has_pending_word_message = true;
    }

    fn score_normal_word(&mut self, word: &str, length: usize) {
        self.remove_visits_for_word(word);
        self.n_words_found += 1;
        self.n_words_found_dirty = true;
        self.n_letters_found += length;
        self.n_letters_found_dirty = true;
        self.update_hint_level();
        self.word_lists_dirty |= 1 << length;
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
                    WordType::Excluded => self.pending_excluded_word = true,
                }
            } else {
                self.save_state_dirty = true;

                match word_data.word_type {
                    WordType::Bonus => self.show_word_message("Bonus word!"),
                    WordType::Normal => {
                        show_word_message!(self, "+{} points!", length);
                        self.score_normal_word(word, length);

                        if self.n_words_found >= self.total_n_words {
                            self.pending_finish = true;
                        }
                    }
                    WordType::Excluded => self.pending_excluded_word = true,
                }
            }
        } else {
            self.show_word_message("Not in list");
            self.misses += 1;
            self.save_state_dirty = true;
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

    pub fn pending_excluded_word(&mut self) -> bool {
        std::mem::replace(&mut self.pending_excluded_word, false)
    }

    pub fn pending_finish(&mut self) -> bool {
        std::mem::replace(&mut self.pending_finish, false)
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

    pub fn changed_save_state(&mut self) -> Option<SaveState> {
        if self.save_state_dirty {
            self.save_state_dirty = false;

            let mut words = self.words.iter()
                .map(|(key, word)| (key, word.found))
                .collect::<Vec<_>>();
            words.sort_unstable_by_key(|&(word, _)| word);

            Some(SaveState::new(
                self.misses,
                self.hints_used,
                words.into_iter().enumerate().filter_map(|(i, (_, found))| {
                    found.then_some(i)
                }),
            ))
        } else {
            None
        }
    }

    pub fn total_n_words(&self) -> usize {
        self.total_n_words
    }

    pub fn total_n_letters(&self) -> usize {
        self.total_n_letters
    }

    pub fn use_hints(&mut self) {
        if !self.hints_used {
            self.hints_used = true;
            self.save_state_dirty = true;
        }
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

    pub fn share_text(&self, puzzle_num: usize) -> String {
        let mut text = format!(
            "I played WordRoute #{}\n\
             {}/{} words",
            puzzle_num,
            self.n_words_found,
            self.total_n_words,
        );

        let n_bonus_words = self.words.values().filter(|word| {
            word.word_type == WordType::Bonus && word.found
        }).count();

        if n_bonus_words > 0 {
            if n_bonus_words == 1 {
                text.push_str(" (+1 bonus word)");
            } else {
                write!(&mut text, " (+{} bonus words)", n_bonus_words).unwrap();
            }
        }

        if self.n_words_found >= self.total_n_words {
            if !self.hints_used {
                text.push_str("\n😎 No hints used");
            }
            if self.misses == 0 {
                text.push_str("\n🎯 Perfect accuracy");
            } else {
                let total_guesses =
                    self.misses as usize +
                    self.total_n_words +
                    n_bonus_words;
                let accuracy = (((total_guesses - self.misses as usize) * 100 +
                                 total_guesses / 2) /
                                total_guesses)
                    .min(99);
                if accuracy >= 75 {
                    write!(&mut text, "\n🎯 {}% accuracy", accuracy).unwrap();
                }
            }
        }

        text
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

fn generate_counts<I, T>(
    grid: &Grid,
    word_finder: &mut word_finder::Finder,
    route_buf: &mut Vec<u8>,
    words: I,
) -> GridCounts
    where I: IntoIterator<Item = T>,
          T: AsRef<str>
{
    let mut counts = GridCounts::new(grid.width(), grid.height());

    for word in words {
        route_buf.clear();

        if let Some((mut x, mut y)) = word_finder.find(
            grid,
            word.as_ref(),
            route_buf,
        ) {
            let start = counts.at_mut(x, y);
            start.starts += 1;
            start.visits += 1;

            for &dir in route_buf.iter() {
                (x, y) = directions::step(x, y, dir);

                counts.at_mut(x, y).visits += 1;
            }
        }
    }

    counts
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

        Puzzle::new(
            grid,
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

        assert_eq!(puzzle.counts().at(0, 0).starts, 0);
        assert_eq!(puzzle.counts().at(0, 0).visits, 0);
        assert_eq!(puzzle.counts().at(0, 1).starts, 0);
        assert_eq!(puzzle.counts().at(0, 1).visits, 0);

        assert!(puzzle.changed_counts().next().is_none());

        assert_eq!(
            &puzzle.changed_word_lists().collect::<Vec<_>>(),
            &[5, 6],
        );
        assert!(puzzle.changed_word_lists().next().is_none());
    }

    #[test]
    fn save_state() {
        let mut puzzle = four_line_puzzle();

        assert!(puzzle.changed_save_state().is_none());

        puzzle.score_word("paobtcadteofsgthoimjpkwlhminposp");
        assert_eq!(
            puzzle.changed_save_state().unwrap().to_string(),
            "0.0.1",
        );
        assert!(puzzle.changed_save_state().is_none());

        puzzle.score_word("paobtcadteofsgthoimjpkwlhminposp");
        assert!(puzzle.changed_save_state().is_none());

        puzzle.use_hints();
        assert_eq!(
            puzzle.changed_save_state().unwrap().to_string(),
            "0.1.1",
        );
        assert!(puzzle.changed_save_state().is_none());

        puzzle.use_hints();
        assert!(puzzle.changed_save_state().is_none());

        puzzle.score_word("missingword");
        assert_eq!(
            puzzle.changed_save_state().unwrap().to_string(),
            "1.1.1",
        );
        assert!(puzzle.changed_save_state().is_none());

        puzzle.score_word("whips");
        assert_eq!(
            puzzle.changed_save_state().unwrap().to_string(),
            "1.1.9",
        );
        assert!(puzzle.changed_save_state().is_none());
    }

    #[test]
    fn load_save_state() {
        let mut puzzle = four_line_puzzle();

        assert_eq!(puzzle.changed_word_lists().count(), 2);
        assert_eq!(puzzle.changed_counts().count(), 64);
        assert_eq!(puzzle.changed_n_words_found().unwrap(), 0);
        assert_eq!(puzzle.changed_n_letters_found().unwrap(), 0);
        assert_eq!(puzzle.changed_hint_level().unwrap(), 0);

        puzzle.load_save_state(&"5.1.2".parse::<SaveState>().unwrap());

        assert!(puzzle.changed_save_state().is_none());

        assert!(puzzle.hints_used);
        assert_eq!(puzzle.misses, 5);

        assert_eq!(
            puzzle.changed_word_lists().collect::<Vec<_>>(),
            &[6],
        );

        assert!(
            puzzle.words.iter().find(|&(key, word)| {
                key == "potato" && word.found
            }).is_some()
        );

        assert_eq!(puzzle.changed_counts().count(), 6);

        assert_eq!(puzzle.changed_n_words_found().unwrap(), 1);
        assert_eq!(puzzle.changed_n_letters_found().unwrap(), 6);
        assert_eq!(puzzle.changed_hint_level().unwrap(), 1);

        puzzle.load_save_state(&"4.1.2".parse::<SaveState>().unwrap());

        assert!(puzzle.hints_used);
        assert_eq!(puzzle.misses, 5);
        assert!(puzzle.changed_word_lists().next().is_none());
        assert!(puzzle.changed_counts().next().is_none());
        assert!(puzzle.changed_n_words_found().is_none());
        assert!(puzzle.changed_n_letters_found().is_none());
        assert!(puzzle.changed_hint_level().is_none());

        puzzle.load_save_state(&"0.0.1".parse::<SaveState>().unwrap());
        assert!(puzzle.hints_used);
        assert_eq!(puzzle.misses, 5);
        assert!(puzzle.changed_word_lists().next().is_none());
        assert!(puzzle.changed_counts().next().is_none());
        assert!(puzzle.changed_n_words_found().is_none());
        assert!(puzzle.changed_n_letters_found().is_none());
        assert!(puzzle.changed_hint_level().is_none());
        assert!(
            puzzle.words.iter().find(|&(key, word)| {
                key == "paobtcadteofsgthoimjpkwlhminposp" && word.found
            }).is_some()
        );
    }

    fn wordy_puzzle() -> Puzzle {
        let grid = Grid::new(".or\nabe\n.ts").unwrap();

        Puzzle::new(
            grid,
            vec![
                ("bats".to_string(), WordType::Normal),
                ("best".to_string(), WordType::Normal),
                ("boat".to_string(), WordType::Normal),
                ("boats".to_string(), WordType::Normal),
                ("bore".to_string(), WordType::Normal),
                ("bores".to_string(), WordType::Normal),
                ("brest".to_string(), WordType::Excluded),
                ("estab".to_string(), WordType::Bonus),
                ("oats".to_string(), WordType::Normal),
                ("robe".to_string(), WordType::Normal),
                ("robes".to_string(), WordType::Normal),
                ("robs".to_string(), WordType::Normal),
                ("sebat".to_string(), WordType::Bonus),
            ]
        )
    }

    #[test]
    fn share_text() {
        let mut puzzle = wordy_puzzle();

        assert_eq!(
            puzzle.share_text(12),
            "I played WordRoute #12\n\
             0/10 words",
        );

        puzzle.score_word("bats");
        puzzle.score_word("best");
        puzzle.score_word("estab");

        assert_eq!(
            puzzle.share_text(12),
            "I played WordRoute #12\n\
             2/10 words (+1 bonus word)",
        );

        puzzle.score_word("sebat");

        assert_eq!(
            puzzle.share_text(12),
            "I played WordRoute #12\n\
             2/10 words (+2 bonus words)",
        );

        for word in [
            "boat", "boats", "bore", "bores", "oats", "robe", "robes", "robs",
        ].iter() {
            puzzle.score_word(word);
        }

        assert_eq!(
            puzzle.share_text(6),
            "I played WordRoute #6\n\
             10/10 words (+2 bonus words)\n\
             😎 No hints used\n\
             🎯 Perfect accuracy",
        );

        puzzle.use_hints();

        assert_eq!(
            puzzle.share_text(42),
            "I played WordRoute #42\n\
             10/10 words (+2 bonus words)\n\
             🎯 Perfect accuracy",
        );

        for _ in 0..4 {
            puzzle.score_word("notaword");
        }

        assert_eq!(
            puzzle.share_text(42),
            "I played WordRoute #42\n\
             10/10 words (+2 bonus words)\n\
             🎯 75% accuracy",
        );

        puzzle.score_word("stillnotaword");

        assert_eq!(
            puzzle.share_text(42),
            "I played WordRoute #42\n\
             10/10 words (+2 bonus words)",
        );
    }

    #[test]
    fn excluded_word() {
        let mut puzzle = wordy_puzzle();

        assert!(!puzzle.pending_excluded_word());

        puzzle.score_word("brest");

        assert!(puzzle.pending_excluded_word());
        assert!(!puzzle.pending_excluded_word());

        let mut puzzle = wordy_puzzle();

        // Assert that loading a save state that marks an excluded
        // word as found doesn’t set a pending excluded word.
        puzzle.load_save_state(
            &"0.0.40".parse::<SaveState>().unwrap(),
        );

        assert!(!puzzle.pending_excluded_word());

        assert!(
            puzzle.words().find(|(key, word)| {
                key == &"brest" &&
                    word.found &&
                    word.word_type == WordType::Excluded
            }).is_some()
        );
    }

    #[test]
    fn finish() {
        let mut puzzle = four_line_puzzle();

        assert!(!puzzle.pending_finish());

        puzzle.score_word("potato");
        assert!(!puzzle.pending_finish());

        puzzle.score_word("stomp");
        assert!(!puzzle.pending_finish());

        puzzle.score_word("whips");
        assert!(puzzle.pending_finish());
        assert!(!puzzle.pending_finish());

        let mut puzzle = four_line_puzzle();

        // Assert that loading a completed save state doesn’t trigger
        // a finish
        puzzle.load_save_state(
            &"0.0.e".parse::<SaveState>().unwrap(),
        );

        assert!(!puzzle.pending_excluded_word());
        assert_eq!(
            puzzle.changed_n_words_found().unwrap(),
            puzzle.total_n_words(),
        );
    }

    #[test]
    fn counts() {
        let puzzle = wordy_puzzle();

        assert_eq!(puzzle.counts.at(0, 0).starts, 0);
        assert_eq!(puzzle.counts.at(0, 0).visits, 0);

        assert_eq!(puzzle.counts.at(1, 0).starts, 1);
        assert_eq!(puzzle.counts.at(1, 0).visits, 8);

        assert_eq!(puzzle.counts.at(2, 0).starts, 3);
        assert_eq!(puzzle.counts.at(2, 0).visits, 5);

        assert_eq!(puzzle.counts.at(0, 1).starts, 0);
        assert_eq!(puzzle.counts.at(0, 1).visits, 4);

        assert_eq!(puzzle.counts.at(1, 1).starts, 6);
        assert_eq!(puzzle.counts.at(1, 1).visits, 9);

        assert_eq!(puzzle.counts.at(2, 1).starts, 0);
        assert_eq!(puzzle.counts.at(2, 1).visits, 5);

        assert_eq!(puzzle.counts.at(0, 2).starts, 0);
        assert_eq!(puzzle.counts.at(0, 2).visits, 0);

        assert_eq!(puzzle.counts.at(1, 2).starts, 0);
        assert_eq!(puzzle.counts.at(1, 2).visits, 5);

        assert_eq!(puzzle.counts.at(2, 2).starts, 0);
        assert_eq!(puzzle.counts.at(2, 2).visits, 7);
    }
}
