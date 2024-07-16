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

#[cfg(target_arch = "wasm32")]
mod wasm_game;
#[cfg(any(target_arch = "wasm32", test))]
mod grid;
#[cfg(any(target_arch = "wasm32", test))]
mod grid_math;
#[cfg(any(target_arch = "wasm32", test))]
mod counts;
#[cfg(any(target_arch = "wasm32", test))]
mod directions;
#[cfg(any(target_arch = "wasm32", test))]
mod word_finder;
