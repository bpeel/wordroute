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

#[derive(Debug)]
pub struct GridCounts {
    values: Box<[TileCounts]>,
    width: u32,
}

#[derive(Clone, Debug)]
pub struct TileCounts {
    pub starts: u8,
    pub visits: u8,
}

impl GridCounts {
    pub fn new(width: u32, height: u32) -> GridCounts {
        GridCounts {
            values: vec![
                TileCounts {
                    starts: 0,
                    visits: 0,
                };
                (width * height) as usize
            ].into_boxed_slice(),
            width,
        }
    }

    pub fn at(&self, x: u32, y: u32) -> &TileCounts {
        assert!(x < self.width);

        &self.values[(y * self.width + x) as usize]
    }

    pub fn at_mut(&mut self, x: u32, y: u32) -> &mut TileCounts {
        assert!(x < self.width);

        &mut self.values[(y * self.width + x) as usize]
    }
}
