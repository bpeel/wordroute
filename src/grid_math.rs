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

// Return the width of the grid as the number of half hexagons. The
// odd rows can take up an extra half hexagon, but sometimes this
// isn’t needed if the end as a blank.
fn half_grid_width(grid: &Grid) -> u32 {
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

pub struct Geometry {
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
    pub fn new(grid: &Grid, viewport_size: f32) -> Geometry {
        // Number of apothems required for the width
        let width_in_apothems = half_grid_width(grid) as f32;
        // The radius of a hexagon in units of apothems
        let radius_in_apothems = 1.0 / (PI / 6.0).cos();
        // Number of apothems required for the height
        let height_in_apothems =
            (grid.height() - 1) as f32 * 1.5 * radius_in_apothems +
            radius_in_apothems * 2.0;

        let apothem = viewport_size / width_in_apothems.max(height_in_apothems);
        let radius = radius_in_apothems * apothem;

        let top_x = viewport_size / 2.0 -
            width_in_apothems * apothem / 2.0 +
            apothem;
        let top_y = viewport_size / 2.0 -
            height_in_apothems * apothem / 2.0 +
            radius;

        Geometry {
            top_x,
            top_y,
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

    #[test]
    fn geometry() {
        let grid = Grid::new("aaaa\naaa.").unwrap();
        let geometry = Geometry::new(&grid, 16.0);

        assert!((geometry.top_x - 2.0).abs() < 0.01);
        assert!((geometry.step_x - 4.0).abs() < 0.01);
        assert!((geometry.radius - (4.0 / 3.0f32.sqrt())).abs() < 0.01);
        assert!((geometry.step_y - (geometry.radius * 1.5)).abs() < 0.01);
        assert!((geometry.top_y - (8.0 - geometry.step_y / 2.0)) < 0.01);
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
}
