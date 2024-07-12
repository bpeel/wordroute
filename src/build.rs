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
use super::directions::DIRECTIONS;
use super::counts::GridCounts;
use super::word_finder;
use std::collections::HashSet;

struct StackEntry<'a> {
    x: u32,
    y: u32,
    walker: dictionary::Walker<'a>,
    next_direction: usize,
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
        if entry.x >= grid.width() ||
            entry.y >= grid.height() ||
            grid.at(entry.x, entry.y) == ' ' ||
            visited[(entry.y * grid.width() + entry.x) as usize] ||
            entry.walker.step(grid.at(entry.x, entry.y)).is_none()
        {
            // Backtrack
            while let Some(entry) = stack.pop() {
                visited[(entry.y * grid.width() + entry.x) as usize] = false;

                if entry.next_direction < DIRECTIONS.len() {
                    stack.push(entry);
                    break;
                }
            }
        } else {
            let letter = grid.at(entry.x, entry.y);
            let next_walker = entry.walker.step(letter).unwrap();

            visited[(entry.y * grid.width() + entry.x) as usize] = true;

            let word_length = stack.len() + 1;

            if word_length >= minimum_length && next_walker.is_end() {
                let mut word = stack.iter().map(|entry| {
                    grid.at(entry.x, entry.y)
                }).collect::<String>();
                word.push(letter);
                word_list.insert(word);
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

    for word in words {
        let route = finder.find(grid, word.as_ref()).unwrap();

        let start = counts.at_mut(route.start_x, route.start_y);
        start.starts += 1;
        start.visits += 1;

        let mut x = route.start_x;
        let mut y = route.start_y;

        for &step in route.steps {
            let offset = DIRECTIONS[step as usize];
            x = x.checked_add_signed(offset.0).unwrap();
            y = y.checked_add_signed(offset.1).unwrap();
            counts.at_mut(x, y).visits += 1;
        }
    }

    counts
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
        assert_eq!(&search("cab", 3), &["cab"]);
        assert_eq!(&search("start", 3), &["start"]);
        assert_eq!(
            &search(
                "cabs\n\
                 trat",
                3,
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
                3,
            ).is_empty(),
        );
        assert_eq!(
            &search(
                " st\n\
                 tra",
                3,
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
                3,
            ),
            &["cab", "start"],
        );
    }

    #[test]
    fn all_directions() {
        assert_eq!(&search("start", 3), &["start"]);
        assert_eq!(&search("trats", 3), &["start"]);
        assert_eq!(&search("s\nt\na\nr\nt", 3), &["start"]);
        assert_eq!(&search("t\nr\na\nt\ns", 3), &["start"]);

        assert_eq!(
            &search(
                "cxx\n\
                 xax\n\
                 xxb",
                3,
            ),
            &["cab"],
        );
        assert_eq!(
            &search(
                "xxc\n\
                 xax\n\
                 bxx",
                3,
            ),
            &["cab"],
        );
        assert_eq!(
            &search(
                "xxb\n\
                 xax\n\
                 cxx",
                3,
            ),
            &["cab"],
        );
        assert_eq!(
            &search(
                "bxx\n\
                 xax\n\
                 xxc",
                3,
            ),
            &["cab"],
        );
    }

    #[test]
    fn minimum_length() {
        assert!(&search("cab", 4).is_empty());
        assert_eq!(&search("cab", 3), &["cab"]);
    }

    #[test]
    fn visits() {
        let grid = Grid::new(
            "stx\n\
             rax\n\
             ctb"
        ).unwrap();

        let words = search_words(&grid, &make_dictionary(), 3);

        assert!(words.contains("start"));
        assert!(words.contains("cab"));

        let counts = count_visits(&grid, words.iter());

        assert_eq!(counts.at(0, 0).starts, 1);
        assert_eq!(counts.at(0, 0).visits, 1);
        assert_eq!(counts.at(1, 0).starts, 0);
        assert_eq!(counts.at(1, 0).visits, 1);
        assert_eq!(counts.at(2, 0).starts, 0);
        assert_eq!(counts.at(2, 0).visits, 0);
        assert_eq!(counts.at(0, 1).starts, 0);
        assert_eq!(counts.at(0, 1).visits, 1);
        assert_eq!(counts.at(1, 1).starts, 0);
        assert_eq!(counts.at(1, 1).visits, 2);
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
