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

// Return the width of the grid as the number of half hexagons. The
// odd rows can take up an extra half hexagon, but sometimes this
// isn’t needed if the end as a blank.
pub fn half_grid_width(grid: &Grid) -> u32 {
    (0..grid.height()).map(|y| {
        let last = (0..grid.width()).rev().find(|&x| grid.at(x, y) != '.')
            .unwrap_or(0);
        let width = (last + 1) * 2;

        if y & 1 == 0 {
            width
        } else {
            width + 1
        }
    }).max().unwrap_or(0)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_half_grid_width() {
        assert_eq!(
            half_grid_width(&Grid::new(
                "a a a\n\
                  a a a"
            ).unwrap()),
            7,
        );
        assert_eq!(
            half_grid_width(&Grid::new(
                "a a a\n\
                  a a ."
            ).unwrap()),
            6,
        );
        assert_eq!(
            half_grid_width(&Grid::new(
                "a a .\n\
                  a a ."
            ).unwrap()),
            5,
        );
        assert_eq!(
            half_grid_width(&Grid::new(
                "a a .\n\
                  a a a"
            ).unwrap()),
            7,
        );
    }
}
