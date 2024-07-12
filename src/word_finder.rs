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
use super::directions::{self, N_DIRECTIONS};

struct StackEntry {
    x: u32,
    y: u32,
    next_direction: u8,
    word_start: usize,
}

pub struct Finder {
    stack: Vec<StackEntry>,
    route: Vec<u8>,
    visited: Vec<bool>,
}

pub struct Route<'a> {
    pub start_x: u32,
    pub start_y: u32,
    pub steps: &'a [u8],
}

impl Finder {
    pub fn new() -> Finder {
        Finder {
            stack: Vec::new(),
            route: Vec::new(),
            visited: Vec::new(),
        }
    }

    fn find_from_position(
        &mut self,
        grid: &Grid,
        word: &str,
        start_x: u32, start_y: u32,
    ) -> bool {
        self.stack.clear();
        self.stack.push(StackEntry {
            x: start_x,
            y: start_y,
            next_direction: 0,
            word_start: 0,
        });

        self.visited.clear();
        self.visited.resize((grid.width() * grid.height()) as usize, false);

        while let Some(mut entry) = self.stack.pop() {
            let letter = word.split_at(entry.word_start).1.chars().next();

            if entry.x >= grid.width() ||
                entry.y >= grid.height() ||
                self.visited[(entry.y * grid.width() + entry.x) as usize] ||
                Some(grid.at(entry.x, entry.y)) != letter
            {
                // Backtrack
                while let Some(entry) = self.stack.pop() {
                    self.visited[
                        (entry.y * grid.width() + entry.x) as usize
                    ] = false;

                    if entry.next_direction < N_DIRECTIONS {
                        self.stack.push(entry);
                        break;
                    }
                }
            } else {
                self.visited[
                    (entry.y * grid.width() + entry.x) as usize
                ] = true;

                let next_word_start =
                    entry.word_start + letter.unwrap().len_utf8();

                if word.split_at(next_word_start).1.is_empty() {
                    self.route.clear();
                    self.route.extend(
                        self.stack.iter().map(|entry| {
                            entry.next_direction as u8 - 1
                        })
                    );
                    return true;
                }

                let next_pos = directions::step(
                    entry.x,
                    entry.y,
                    entry.next_direction,
                );

                let next_entry = StackEntry {
                    x: next_pos.0,
                    y: next_pos.1,
                    word_start: next_word_start,
                    next_direction: 0,
                };

                entry.next_direction += 1;
                self.stack.push(entry);

                self.stack.push(next_entry);
            }
        }

        false
    }

    pub fn find(&mut self, grid: &Grid, word: &str) -> Option<Route> {
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                if self.find_from_position(grid, word, x, y) {
                    return Some(Route {
                        start_x: x,
                        start_y: y,
                        steps: &self.route,
                    });
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn all_directions() {
        let mut finder = Finder::new();

        let grid = Grid::new(
            "a b c\n\
              d e f\n\
             g h i"
        ).unwrap();

        let route = finder.find(&grid, "abc").unwrap();
        assert_eq!(route.start_x, 0);
        assert_eq!(route.start_y, 0);
        assert_eq!(route.steps, &[3, 3]);

        let route = finder.find(&grid, "cba").unwrap();
        assert_eq!(route.start_x, 2);
        assert_eq!(route.start_y, 0);
        assert_eq!(route.steps, &[2, 2]);

        let route = finder.find(&grid, "adg").unwrap();
        assert_eq!(route.start_x, 0);
        assert_eq!(route.start_y, 0);
        assert_eq!(route.steps, &[5, 4]);

        let route = finder.find(&grid, "gda").unwrap();
        assert_eq!(route.start_x, 0);
        assert_eq!(route.start_y, 2);
        assert_eq!(route.steps, &[1, 0]);
    }

    #[test]
    fn backtrack() {
        let mut finder = Finder::new();

        let grid = Grid::new(
            "b a c k t r a p\n\
              x x x x x x c k"
        ).unwrap();

        let route = finder.find(&grid, "backtrap").unwrap();
        assert_eq!(route.start_x, 0);
        assert_eq!(route.start_y, 0);
        assert_eq!(route.steps, &[3, 3, 3, 3, 3, 3, 3]);

        let route = finder.find(&grid, "backtrack").unwrap();
        assert_eq!(route.start_x, 0);
        assert_eq!(route.start_y, 0);
        assert_eq!(route.steps, &[3, 3, 3, 3, 3, 3, 5, 3]);
    }

    #[test]
    fn not_found() {
        let mut finder = Finder::new();
        let grid = Grid::new("haystack").unwrap();
        assert!(finder.find(&grid, "needle").is_none());
    }

    #[test]
    fn no_reuse() {
        let mut finder = Finder::new();
        let grid = Grid::new(
            "r e u\n\
              e s x"
        ).unwrap();

        // Make sure that the bottom ‘e’ was used for the last letter
        // instead of reusing the top ‘e’.

        let route = finder.find(&grid, "reuse").unwrap();
        assert_eq!(route.start_x, 0);
        assert_eq!(route.start_y, 0);
        assert_eq!(route.steps, &[3, 3, 4, 2]);

        assert!(finder.find(&grid, "reuser").is_none());
    }
}
