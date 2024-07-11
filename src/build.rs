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
use std::collections::HashSet;

static DIRECTIONS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),

    (-1, 0),
    (1, 0),

    (-1, 1),
    (0, 1),
    (1, 1),
];

struct StackEntry<'a> {
    x: u32,
    y: u32,
    walker: dictionary::Walker<'a>,
    next_direction: usize,
}

fn search_from_pos(
    grid: &Grid,
    dictionary: &dictionary::Dictionary,
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
    let mut word = String::new();

    while let Some(mut entry) = stack.pop() {
        if entry.x >= grid.width() ||
            entry.y >= grid.height() ||
            grid.at(entry.x, entry.y) == ' ' ||
            visited[(entry.y * grid.width() + entry.x) as usize] ||
            entry.walker.step(grid.at(entry.x, entry.y)).is_none()
        {
            // Backtrack
            while let Some(entry) = stack.pop() {
                visited[(entry.y * grid.width() + entry.x) as usize] = false;
                word.pop().unwrap();

                if entry.next_direction < DIRECTIONS.len() {
                    stack.push(entry);
                    break;
                }
            }
        } else {
            let letter = grid.at(entry.x, entry.y);
            let next_walker = entry.walker.step(letter).unwrap();

            word.push(letter);
            visited[(entry.y * grid.width() + entry.x) as usize] = true;

            if next_walker.is_end() {
                word_list.insert(word.clone());
            }

            let next_offset = DIRECTIONS[entry.next_direction];

            let next_entry = StackEntry {
                x: entry.x.wrapping_add_signed(next_offset.0),
                y: entry.y.wrapping_add_signed(next_offset.1),
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
) -> HashSet<String> {
    let mut word_list = HashSet::new();

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            search_from_pos(grid, dictionary, x, y, &mut word_list);
        }
    }

    word_list
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_dictionary() -> dictionary::Dictionary {
        static DICTIONARY_BYTES: [u8; 33] = [
            0x00, 0x01, b'*',
            0x0a, 0x01, b'c',
            0x00, 0x01, b'a',
            0x00, 0x01, b'b',
            0x00, 0x00, b'\0',
            0x00, 0x01, b's',
            0x00, 0x01, b't',
            0x00, 0x01, b'a',
            0x00, 0x01, b'r',
            0x00, 0x01, b't',
            0x00, 0x00, b'\0',
        ];

        dictionary::Dictionary::new(Box::new(DICTIONARY_BYTES.clone()))
    }

    fn search(grid: &str) -> Vec<String> {
        let mut words = search_words(
            &Grid::new(grid).unwrap(),
            &make_dictionary(),
        ).into_iter().collect::<Vec<_>>();

        words.sort_unstable();

        words
    }

    #[test]
    fn simple() {
        assert_eq!(&search("cab"), &["cab"]);
        assert_eq!(&search("start"), &["start"]);
        assert_eq!(
            &search(
                "cabs\n\
                 trat"
            ),
            &["cab", "start"],
        );
    }

    #[test]
    fn no_reuse() {
        assert!(
            &search(
                " st\n\
                 xra",
            ).is_empty(),
        );
        assert_eq!(
            &search(
                " st\n\
                 tra",
            ),
            &["start"],
        );
    }

    #[test]
    fn cross() {
        assert_eq!(
            &search(
                "  c  \n\
                 start\n\
                 xxb",
            ),
            &["cab", "start"],
        );
    }

    #[test]
    fn all_directions() {
        assert_eq!(&search("start"), &["start"]);
        assert_eq!(&search("trats"), &["start"]);
        assert_eq!(&search("s\nt\na\nr\nt"), &["start"]);
        assert_eq!(&search("t\nr\na\nt\ns"), &["start"]);

        assert_eq!(
            &search(
                "cxx\n\
                 xax\n\
                 xxb",
            ),
            &["cab"],
        );
        assert_eq!(
            &search(
                "xxc\n\
                 xax\n\
                 bxx",
            ),
            &["cab"],
        );
        assert_eq!(
            &search(
                "xxb\n\
                 xax\n\
                 cxx",
            ),
            &["cab"],
        );
        assert_eq!(
            &search(
                "bxx\n\
                 xax\n\
                 xxc",
            ),
            &["cab"],
        );
    }
}
