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

// The grid is arranged in hexagons as if every other row is shifted
// right by half a position. That means that for example on even rows
// (where the first “zeroth” row is even) the downward directions are
// the coordinate directly below and the coordinate down and left,
// whereas on odd rows they are the coordinate directly below and the
// coordinate down and right.

// a b c d
//  e f g h
// i j k l

pub const N_DIRECTIONS: u8 = 6;

pub fn step(x: u32, y: u32, direction: u8) -> (u32, u32) {
    let y_off;

    if direction < 2 {
        y_off = -1;
    } else if direction < 4 {
        let x_off = if direction & 1 == 0 {
            -1
        } else {
            1
        };

        return (x.wrapping_add_signed(x_off), y);
    } else {
        assert!(direction < N_DIRECTIONS);
        y_off = 1;
    }

    let x_off = (direction & 1) as i32 - 1 + (y & 1) as i32;

    (x.wrapping_add_signed(x_off), y.wrapping_add_signed(y_off))
}

#[cfg(any(target_arch = "wasm32", test))]
// Given a position and the direction that was used to get there,
// return the starting position.
pub fn reverse(x: u32, y: u32, direction: u8) -> (u32, u32) {
    step(x, y, 5 - direction)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn step_all_directions() {
        // Even rows
        assert_eq!(step(1, 2, 0), (0, 1));
        assert_eq!(step(1, 2, 1), (1, 1));
        assert_eq!(step(1, 2, 2), (0, 2));
        assert_eq!(step(1, 2, 3), (2, 2));
        assert_eq!(step(1, 2, 4), (0, 3));
        assert_eq!(step(1, 2, 5), (1, 3));
        // Odd rows
        assert_eq!(step(1, 1, 0), (1, 0));
        assert_eq!(step(1, 1, 1), (2, 0));
        assert_eq!(step(1, 1, 2), (0, 1));
        assert_eq!(step(1, 1, 3), (2, 1));
        assert_eq!(step(1, 1, 4), (1, 2));
        assert_eq!(step(1, 1, 5), (2, 2));
    }

    #[test]
    fn overflow() {
        // Going off the top or left of the grid should wrap the
        // coordinates around the integer maximum so that the rest of
        // the program can easily detect invalid directions with just
        // a single comparison against the dimensions of the grid.
        assert_eq!(step(0, 0, 2), (u32::MAX, 0));
        assert_eq!(step(0, 0, 1), (0, u32::MAX));
    }

    #[test]
    fn reverse_matches_step() {
        for dir in 0..N_DIRECTIONS {
            let next = step(2, 1, dir);
            assert_eq!(reverse(next.0, next.1, dir), (2, 1));
        }
    }
}
