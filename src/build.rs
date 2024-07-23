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
use super::dictionary;
use super::directions::{self, N_DIRECTIONS};
use super::counts::GridCounts;
use super::word_finder;
use std::collections::HashSet;

struct StackEntry<'a> {
    x: u32,
    y: u32,
    walker: dictionary::Walker<'a>,
    next_direction: u8,
}

fn search_from_pos(
    grid: &Grid,
    dictionary: &dictionary::Dictionary,
    minimum_length: usize,
    x: u32,
    y: u32,
    word_list: &mut HashSet<String>,
) {
    let Some(walker) = dictionary::Walker::new(dictionary)
    else {
        return;
    };

    let mut stack = vec![StackEntry {
        x,
        y,
        walker,
        next_direction: 0,
    }];

    let mut visited = vec![false; (grid.width() * grid.height()) as usize];

    while let Some(mut entry) = stack.pop() {
        if entry.next_direction == 0 &&
            (entry.x >= grid.width() ||
             entry.y >= grid.height() ||
             visited[(entry.y * grid.width() + entry.x) as usize] ||
             entry.walker.step(grid.at(entry.x, entry.y)).is_none())
        {
            // Backtrack
            while let Some(entry) = stack.pop() {
                visited[(entry.y * grid.width() + entry.x) as usize] = false;

                if entry.next_direction < N_DIRECTIONS {
                    stack.push(entry);
                    break;
                }
            }
        } else {
            let letter = grid.at(entry.x, entry.y);
            let next_walker = entry.walker.step(letter).unwrap();

            visited[(entry.y * grid.width() + entry.x) as usize] = true;

            let word_length = stack.len() + 1;

            if entry.next_direction == 0 &&
                word_length >= minimum_length &&
                next_walker.is_end()
            {
                let mut word = stack.iter().map(|entry| {
                    grid.at(entry.x, entry.y)
                }).collect::<String>();
                word.push(letter);
                word_list.insert(word);
            }

            let next_pos = directions::step(
                entry.x,
                entry.y,
                entry.next_direction,
            );

            let next_entry = StackEntry {
                x: next_pos.0,
                y: next_pos.1,
                walker: next_walker,
                next_direction: 0,
            };

            entry.next_direction += 1;
            stack.push(entry);

            stack.push(next_entry);
        }
    }
}

pub fn search_words(
    grid: &Grid,
    dictionary: &dictionary::Dictionary,
    minimum_length: usize,
) -> HashSet<String> {
    let mut word_list = HashSet::new();

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            search_from_pos(
                grid,
                dictionary,
                minimum_length,
                x, y,
                &mut word_list,
            );
        }
    }

    word_list
}

pub fn count_visits<I, T>(
    grid: &Grid,
    words: I,
) -> GridCounts
    where I: IntoIterator<Item = T>,
          T: AsRef<str>
{
    let mut counts = GridCounts::new(grid.width(), grid.height());
    let mut finder = word_finder::Finder::new();
    let mut steps = Vec::new();

    for word in words {
        steps.clear();

        let (mut x, mut y) =
            finder.find(grid, word.as_ref(), &mut steps).unwrap();

        let start = counts.at_mut(x, y);
        start.starts += 1;
        start.visits += 1;

        for &step in steps.iter() {
            (x, y) = directions::step(x, y, step);
            counts.at_mut(x, y).visits += 1;
        }
    }

    counts
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_dictionary() -> dictionary::Dictionary {
        // Dictonary with the words 𐑕𐑑𐑨𐑓𐑑 and 𐑒𐑨𐑚
        static DICTIONARY_BYTES: [u8; 57] = [
            0x00, 0x01, b'*',
            0x13, 0x04, 0xf0, 0x90, 0x91, 0x92, // 𐑒
            0x00, 0x04, 0xf0, 0x90, 0x91, 0xa8, // 𐑨
            0x00, 0x04, 0xf0, 0x90, 0x91, 0x9a, // 𐑚
            0x00, 0x00, b'\0',
            0x00, 0x04, 0xf0, 0x90, 0x91, 0x95, // 𐑕
            0x00, 0x04, 0xf0, 0x90, 0x91, 0x91, // 𐑑
            0x00, 0x04, 0xf0, 0x90, 0x91, 0xa8, // 𐑨
            0x00, 0x04, 0xf0, 0x90, 0x91, 0x93, // 𐑓
            0x00, 0x04, 0xf0, 0x90, 0x91, 0x91, // 𐑑
            0x00, 0x00, b'\0',
        ];

        dictionary::Dictionary::new(Box::new(DICTIONARY_BYTES.clone()))
    }

    fn search(grid: &str, minimum_length: usize) -> Vec<String> {
        let mut words = search_words(
            &Grid::new(grid).unwrap(),
            &make_dictionary(),
            minimum_length,
        ).into_iter().collect::<Vec<_>>();

        words.sort_unstable();

        words
    }

    #[test]
    fn simple() {
        assert_eq!(&search("𐑒𐑨𐑚", 3), &["𐑒𐑨𐑚"]);
        assert_eq!(&search("𐑕𐑑𐑨𐑓𐑑", 3), &["𐑕𐑑𐑨𐑓𐑑"]);
        assert_eq!(
            &search(
                " 𐑒 𐑨 𐑚 𐑕\
                 : 𐑑 𐑓 𐑨 𐑑",
                3,
            ),
            &["𐑒𐑨𐑚", "𐑕𐑑𐑨𐑓𐑑"],
        );
    }

    #[test]
    fn no_reuse() {
        assert!(
            &search(
                " . 𐑕 𐑑\
                 : 𐑿 𐑓 𐑨",
                3,
            ).is_empty(),
        );
        assert_eq!(
            &search(
                " . 𐑕 𐑑\
                 : 𐑑 𐑓 𐑨",
                3,
            ),
            &["𐑕𐑑𐑨𐑓𐑑"],
        );
    }

    #[test]
    fn cross() {
        assert_eq!(
            &search(
                " . . 𐑒 . .\
                 : 𐑕 𐑑 𐑨 𐑓 𐑑\
                 :𐑿 𐑿 𐑚",
                3,
            ),
            &["𐑒𐑨𐑚", "𐑕𐑑𐑨𐑓𐑑"],
        );
    }

    #[test]
    fn all_directions() {
        assert_eq!(&search("𐑕𐑑𐑨𐑓𐑑", 3), &["𐑕𐑑𐑨𐑓𐑑"]);
        assert_eq!(&search("𐑑𐑓𐑨𐑑𐑕", 3), &["𐑕𐑑𐑨𐑓𐑑"]);

        assert_eq!(
            &search(
                " 𐑕 x x\
                 : 𐑑 x x\
                 :x 𐑨 x\
                 : x 𐑓 x\
                 :x x 𐑑",
                3,
            ),
            &["𐑕𐑑𐑨𐑓𐑑"]);
        assert_eq!(
            &search(
                " x x 𐑕\
                 : x 𐑑 x\
                 :x 𐑨 x\
                 : 𐑓 x x\
                 :𐑑 x x",
                3,
            ),
            &["𐑕𐑑𐑨𐑓𐑑"]);

        assert_eq!(
            &search(
                " 𐑑 x x\
                 : 𐑓 x x\
                 :x 𐑨 x\
                 : x 𐑑 x\
                 :x x 𐑕",
                3,
            ),
            &["𐑕𐑑𐑨𐑓𐑑"]);
        assert_eq!(
            &search(
                " x x 𐑑\
                 : x 𐑓 x\
                 :x 𐑨 x\
                 : 𐑑 x x\
                 :𐑕 x x",
                3,
            ),
            &["𐑕𐑑𐑨𐑓𐑑"]);
    }

    #[test]
    fn minimum_length() {
        assert!(&search("𐑒𐑨𐑚", 4).is_empty());
        assert_eq!(&search("𐑒𐑨𐑚", 3), &["𐑒𐑨𐑚"]);
    }

    #[test]
    fn visits() {
        let grid = Grid::new(
            " 𐑕 𐑑 x\
             : 𐑨 𐑓 x\
             :𐑒 𐑚 𐑑"
        ).unwrap();

        let words = search_words(&grid, &make_dictionary(), 3);

        assert!(words.contains("𐑕𐑑𐑨𐑓𐑑"));
        assert!(words.contains("𐑒𐑨𐑚"));

        let counts = count_visits(&grid, words.iter());

        assert_eq!(counts.at(0, 0).starts, 1);
        assert_eq!(counts.at(0, 0).visits, 1);
        assert_eq!(counts.at(1, 0).starts, 0);
        assert_eq!(counts.at(1, 0).visits, 1);
        assert_eq!(counts.at(2, 0).starts, 0);
        assert_eq!(counts.at(2, 0).visits, 0);
        assert_eq!(counts.at(0, 1).starts, 0);
        assert_eq!(counts.at(0, 1).visits, 2);
        assert_eq!(counts.at(1, 1).starts, 0);
        assert_eq!(counts.at(1, 1).visits, 1);
        assert_eq!(counts.at(2, 1).starts, 0);
        assert_eq!(counts.at(2, 1).visits, 0);
        assert_eq!(counts.at(0, 2).starts, 1);
        assert_eq!(counts.at(0, 2).visits, 1);
        assert_eq!(counts.at(1, 2).starts, 0);
        assert_eq!(counts.at(1, 2).visits, 1);
        assert_eq!(counts.at(2, 2).starts, 0);
        assert_eq!(counts.at(2, 2).visits, 1);
    }
}
