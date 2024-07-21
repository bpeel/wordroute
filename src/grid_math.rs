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
use std::f32::consts::PI;

// Return the start and end of the grid in units of of half
// hexagons. The odd rows can take up an extra half hexagon, but
// sometimes this isn’t needed if the end as a blank.
fn half_grid_size(grid: &Grid) -> (u32, u32) {
    (0..grid.height()).map(|y| {
        let first = (0..grid.width()).find(|&x| grid.at(x, y) != '.')
            .unwrap_or(grid.width() - 1) *
            2;
        let last = ((0..grid.width()).rev().find(|&x| grid.at(x, y) != '.')
                    .unwrap_or(0) +
                    1) *
            2;

        if y & 1 == 0 {
            (first, last)
        } else {
            (first + 1, last + 1)
        }
    }).fold(
        (u32::MAX, 0),
        |(first_a, last_a), (first_b, last_b)| {
            (first_a.min(first_b), last_a.max(last_b))
        },
    )
}

pub struct Geometry {
    pub width: f32,
    pub height: f32,
    // Coordinates of the center of the top left hexagon
    pub top_x: f32,
    pub top_y: f32,
    // The outer radius of a hexagon
    pub radius: f32,
    // Horizontal distance between the centres of hexagons
    pub step_x: f32,
    // Vertical dintance between the centres of hexagons
    pub step_y: f32,
}

impl Geometry {
    pub fn new(grid: &Grid, viewport_width: f32) -> Geometry {
        let (first, last) = half_grid_size(grid);
        // Number of apothems required for the width
        let width_in_apothems = (last - first) as f32;
        // The radius of a hexagon in units of apothems
        let radius_in_apothems = 1.0 / (PI / 6.0).cos();
        // Number of apothems required for the height
        let height_in_apothems =
            (grid.height() - 1) as f32 * 1.5 * radius_in_apothems +
            radius_in_apothems * 2.0;

        let apothem = viewport_width / width_in_apothems;
        let radius = radius_in_apothems * apothem;

        Geometry {
            width: viewport_width,
            height: apothem * height_in_apothems,
            top_x: apothem - first as f32 * apothem,
            top_y: radius,
            radius,
            step_x: apothem * 2.0,
            step_y: radius * 1.5,
        }
    }

    // Calculate the centre of a hexagon in the grid
    pub fn convert_coords(&self, x: u32, y: u32) -> (f32, f32) {
        let x_off = if y & 1 == 0 {
            0.0
        } else {
            self.step_x / 2.0
        };

        (
            self.top_x + x as f32 * self.step_x + x_off,
            self.top_y + y as f32 * self.step_y,
        )
    }

    // Return the hexagon that covers the given coordinates, if there is one
    pub fn reverse_coords(&self, x: f32, y: f32) -> (u32, u32) {
        if x < 0.0 || y < 0.0 {
            return (u32::MAX, u32::MAX);
        }

        // Offset the y from the top of the points of the top row
        let y = y - (self.top_y - self.radius);
        // Offset the x from the leftmost straight part of the grid
        let x = x - (self.top_x - self.step_x / 2.0);

        // Half the height of the rectangular part in the middle of the hexagon
        let half_rect_height = self.radius * 0.5;

        let row_height = self.radius -
            half_rect_height +
            half_rect_height * 2.0;

        let mut grid_y = (y / row_height) as u32;

        let triangle_height = self.radius - half_rect_height;
        let y_in_row = y % row_height;

        // Are we in the bit of the grid with the wavy line?
        if y_in_row < triangle_height {
            let x_in_hex = if grid_y & 1 == 0 {
                x % self.step_x
            } else {
                (x + self.step_x / 2.0) % self.step_x
            };

            let row_start = if x_in_hex < self.step_x / 2.0 {
                triangle_height -
                    x_in_hex * triangle_height / (self.step_x / 2.0)
            } else {
                (x_in_hex - self.step_x / 2.0) *
                    triangle_height /
                    (self.step_x / 2.0)
            };

            if y_in_row < row_start {
                grid_y = grid_y.wrapping_add_signed(-1)
            }
        }

        let mut grid_x = (x / self.step_x) as u32;

        if grid_y & 1 == 1 && x % self.step_x < self.step_x / 2.0 {
            grid_x = grid_x.wrapping_add_signed(-1);
        };

        (grid_x, grid_y)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_half_grid_width() {
        assert_eq!(
            half_grid_size(&Grid::new(
                "a a a\n\
                  a a a"
            ).unwrap()),
            (0, 7),
        );
        assert_eq!(
            half_grid_size(&Grid::new(
                "a a a\n\
                  a a ."
            ).unwrap()),
            (0, 6),
        );
        assert_eq!(
            half_grid_size(&Grid::new(
                "a a .\n\
                  a a ."
            ).unwrap()),
            (0, 5),
        );
        assert_eq!(
            half_grid_size(&Grid::new(
                "a a .\n\
                  a a a"
            ).unwrap()),
            (0, 7),
        );
        assert_eq!(
            half_grid_size(&Grid::new(
                ". a a\n\
                  a a a\n\
                 . a a"
            ).unwrap()),
            (1, 7),
        );
    }

    #[test]
    fn geometry() {
        let grid = Grid::new("aaaa\naaa.").unwrap();
        let geometry = Geometry::new(&grid, 16.0);

        assert!((geometry.top_x - 2.0).abs() < 0.01);
        assert!((geometry.step_x - 4.0).abs() < 0.01);
        assert!((geometry.radius - (4.0 / 3.0f32.sqrt())).abs() < 0.01);
        assert!((geometry.step_y - (geometry.radius * 1.5)).abs() < 0.01);
        assert!((geometry.top_y - geometry.radius).abs() < 0.01);

        let grid = Grid::new(".aa\naaa").unwrap();
        let geometry = Geometry::new(&grid, 21.0);

        assert!(geometry.top_x.abs() < 0.01);
        assert!((geometry.step_x - 7.0).abs() < 0.01);
    }

    #[test]
    fn convert_coords() {
        let grid = Grid::new("aaaa\naaa.").unwrap();
        let geometry = Geometry::new(&grid, 16.0);

        let (center_x, center_y) = geometry.convert_coords(0, 0);

        assert!((center_x - 2.0).abs() < 0.01);
        assert!((geometry.top_y - center_y).abs() < 0.01);

        let (center_x, center_y) = geometry.convert_coords(1, 1);

        assert!((center_x - 8.0).abs() < 0.01);
        assert!((geometry.top_y + geometry.step_y - center_y).abs() < 0.01);
    }

    #[test]
    fn reverse_coords() {
        let grid = Grid::new(".𐑱𐑖𐑩\n𐑼𐑦𐑤𐑯\n𐑦𐑑𐑟𐑮𐑴\n𐑙𐑯𐑨𐑑\n.𐑒𐑼𐑟").unwrap();
        let geometry = Geometry::new(&grid, 100.0);

        // Outside top left of hexagon
        assert_eq!(geometry.reverse_coords(24.77678, 1.33928), (0, 4294967295));
        // Outside top right of hexagon
        assert_eq!(geometry.reverse_coords(45.982143, 18.303572), (2, 0));
        // Outside bottom left of hexagon
        assert_eq!(geometry.reverse_coords(43.30357, 54.6875), (1, 3));
        // Outside bottom right of hexagon
        assert_eq!(geometry.reverse_coords(75.22321, 55.13393), (3, 3));
        // Inside middle rectangle of hexagon
        assert_eq!(geometry.reverse_coords(8.03571, 28.34821), (4294967295, 1));
    }
}
