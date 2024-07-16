// Wordroute – A word game
// Copyright (C) 2023, 2024  Neil Roberts
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

use wasm_bindgen::prelude::*;
use web_sys::console;
use super::grid::Grid;
use super::counts::{TileCounts, GridCounts};
use super::grid_math::Geometry;
use super::word_finder;
use super::directions;
use std::fmt::Write;
use js_sys::Reflect;
use std::f32::consts::PI;
use std::collections::HashMap;

const SVG_NAMESPACE: &'static str = "http://www.w3.org/2000/svg";
const ROUTE_ID: &'static str = "route-line";

fn show_error(message: &str) {
    console::log_1(&message.into());

    let Some(window) = web_sys::window()
    else {
        return;
    };

    let Some(document) = window.document()
    else {
        return;
    };

    let Some(message_elem) = document.get_element_by_id("message")
    else {
        return;
    };

    message_elem.set_text_content(Some("An error occurred"));
}

struct Context {
    document: web_sys::HtmlDocument,
    window: web_sys::Window,
    message: web_sys::HtmlElement,
}

impl Context {
    fn new() -> Result<Context, String> {
        let Some(window) = web_sys::window()
        else {
            return Err("failed to get window".to_string());
        };

        let Some(document) = window.document()
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
        else {
            return Err("failed to get document".to_string());
        };

        let Some(message) = document.get_element_by_id("message")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get message div".to_string());
        };

        Ok(Context {
            document,
            window,
            message,
        })
    }
}

type PromiseClosure = Closure::<dyn FnMut(JsValue)>;

struct Loader {
    context: Context,

    data_response_closure: Option<PromiseClosure>,
    data_content_closure: Option<PromiseClosure>,
    data_error_closure: Option<PromiseClosure>,

    floating_pointer: Option<*mut Loader>,
}

impl Loader {
    fn new(context: Context) -> Loader {
        Loader {
            context,
            data_response_closure: None,
            data_content_closure: None,
            data_error_closure: None,
            floating_pointer: None,
        }
    }

    fn start_floating(self) -> *mut Loader {
        assert!(self.floating_pointer.is_none());

        let floating_pointer = Box::into_raw(Box::new(self));

        unsafe {
            (*floating_pointer).floating_pointer = Some(floating_pointer);
        }

        floating_pointer
    }

    fn stop_floating(&mut self) -> Loader {
        match self.floating_pointer {
            Some(floating_pointer) => unsafe {
                // This should end up destroying the loader and
                // invalidating any closures that it holds
                *Box::from_raw(floating_pointer)
            },
            None => unreachable!(),
        }
    }

    fn queue_data_load(&mut self) {
        let filename = "puzzle.json";

        let floating_pointer = self.floating_pointer.unwrap();

        let response_closure = PromiseClosure::new(move |v: JsValue| {
            let (content_closure, error_closure) = unsafe {
                (
                    (*floating_pointer).data_content_closure.as_ref().unwrap(),
                    (*floating_pointer).data_error_closure.as_ref().unwrap(),
                )
            };

            let response: web_sys::Response = v.dyn_into().unwrap();
            let promise = match response.json() {
                Ok(p) => p,
                Err(_) => {
                    show_error("Error fetching json from data");
                    unsafe {
                        (*floating_pointer).stop_floating();
                    }
                    return;
                },
            };
            let _ = promise.then2(content_closure, error_closure);
        });

        let content_closure = PromiseClosure::new(move |v| {
            unsafe {
                (*floating_pointer).data_loaded(v);
            }
        });

        let error_closure = PromiseClosure::new(move |_| {
            show_error("Error loading data");
            unsafe {
                (*floating_pointer).stop_floating();
            }
        });

        let mut request_init = web_sys::RequestInit::new();
        request_init.cache(web_sys::RequestCache::NoCache);

        let promise = self.context.window.fetch_with_str_and_init(
            filename,
            &request_init,
        );

        let _ = promise.then2(&response_closure, &error_closure);

        self.data_response_closure = Some(response_closure);
        self.data_content_closure = Some(content_closure);
        self.data_error_closure = Some(error_closure);
    }

    fn data_loaded(&mut self, data: JsValue) {
        match parse_puzzles(data) {
            Err(_) => {
                self.stop_floating();
            },
            Ok(puzzles) => self.start_game(puzzles),
        }
    }

    fn start_game(&mut self, puzzles: Vec<Puzzle>) {
        let Loader { context, .. } = self.stop_floating();

        match Wordroute::new(context, puzzles) {
            Ok(wordroute) => {
                // Leak the main wordroute object so that it will live as
                // long as the web page
                std::mem::forget(wordroute);
            },
            Err(e) => show_error(&e.to_string()),
        }
    }
}

struct Letter {
    group: web_sys::SvgElement,
    starts: web_sys::SvgElement,
    visits: web_sys::SvgElement,
}

enum WordType {
    Normal,
    Bonus,
}

struct Word {
    word_type: WordType,
    length: usize,
    found: bool,
}

struct Puzzle {
    grid: Grid,
    counts: GridCounts,
    words: HashMap<String, Word>,
}

struct Wordroute {
    context: Context,
    keydown_closure: Option<Closure::<dyn Fn(JsValue)>>,
    game_contents: web_sys::HtmlElement,
    game_grid: web_sys::SvgElement,
    letters: Vec<Letter>,
    grid: Grid,
    counts: GridCounts,
    words: HashMap<String, Word>,
    geometry: Geometry,
    word_finder: word_finder::Finder,
    route_start: Option<(u32, u32)>,
    route_steps: Vec<u8>,
}

impl Wordroute {
    fn new(
        context: Context,
        puzzles: Vec<Puzzle>
    ) -> Result<Box<Wordroute>, String> {
        let Some(game_contents) =
            context.document.get_element_by_id("game-contents")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get game contents".to_string());
        };

        let Some(game_grid) = context.document.get_element_by_id("game-grid")
            .and_then(|c| c.dyn_into::<web_sys::SvgElement>().ok())
        else {
            return Err("failed to get game grid".to_string());
        };

        let Some(Puzzle { grid, counts, words }) = puzzles.into_iter().next()
        else {
            return Err("no puzzles available".to_string());
        };

        let geometry = Geometry::new(&grid, 100.0);

        let mut wordroute = Box::new(Wordroute {
            context,
            keydown_closure: None,
            game_contents,
            game_grid,
            grid,
            counts,
            words,
            geometry,
            letters: Vec::new(),
            word_finder: word_finder::Finder::new(),
            route_start: None,
            route_steps: Vec::new(),
        });

        wordroute.create_closures();
        wordroute.update_title();
        wordroute.create_letters()?;

        wordroute.show_game_contents();

        Ok(wordroute)
    }

    fn create_closures(&mut self) {
        let wordroute_pointer = self as *mut Wordroute;

        let keydown_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                let event: web_sys::KeyboardEvent = event.dyn_into().unwrap();
                wordroute.handle_keydown_event(event);
            }
        );

        let _ = self.context.document.add_event_listener_with_callback(
            "keydown",
            keydown_closure.as_ref().unchecked_ref(),
        );

        self.keydown_closure = Some(keydown_closure);
    }

    fn create_svg_element(
        &self,
        name: &str,
    ) -> Result<web_sys::SvgElement, String> {
        self.context.document.create_element_ns(
            Some(SVG_NAMESPACE),
            name,
        ).ok().and_then(|c| c.dyn_into::<web_sys::SvgElement>().ok())
            .ok_or_else(|| "failed to create letter element".to_string())
    }

    fn create_letter_text(
        &self,
        text: &str,
        y: f32,
        font_size: f32,
    ) -> Result<web_sys::SvgElement, String> {
        let elem = self.create_svg_element("text")?;
        let _ = elem.set_attribute("text-anchor", "middle");
        let _ = elem.set_attribute("x", "0");
        let _ = elem.set_attribute("y", &y.to_string());
        let _ = elem.set_attribute("font-size", &font_size.to_string());

        let text_node = self.context.document.create_text_node(text);
        let _ = elem.append_with_node_1(&text_node);

        Ok(elem)
    }

    fn create_letters(&mut self) -> Result<(), String> {
        let hexagon_path = hexagon_path(self.geometry.radius);

        let font_size = self.geometry.radius * 1.2;
        let text_y_pos = self.geometry.radius * 0.3;

        let counts_font_size = self.geometry.radius * 0.2;

        for (x, y) in (0..self.grid.height())
            .map(|y| (0..self.grid.width()).map(move |x| (x, y)))
            .flatten()
        {
            let letter = self.grid.at(x, y);

            if letter == '.' {
                continue;
            }

            let g = self.create_svg_element("g")?;

            let (x_center, y_center) = self.geometry.convert_coords(x, y);

            let _ = g.set_attribute("class", "letter");
            let _ = g.set_attribute(
                "transform",
                &format!("translate({}, {})", x_center, y_center),
            );
            g.set_id(&format!("letter-{}-{}", x, y));

            let path = self.create_svg_element("path")?;
            let _ = path.set_attribute("d", &hexagon_path);

            let _ = g.append_with_node_1(&path);

            let text = self.create_letter_text(
                &self.grid.at(x, y).to_string(),
                text_y_pos,
                font_size,
            )?;

            let _ = g.append_with_node_1(&text);

            let TileCounts { starts, visits } = self.counts.at(x, y);

            let starts = self.create_letter_text(
                &starts.to_string(),
                -self.geometry.radius * 0.7,
                counts_font_size,
            )?;
            let _ = starts.set_attribute("class", "starts");
            let _ = g.append_with_node_1(&starts);

            let visits = self.create_letter_text(
                &visits.to_string(),
                self.geometry.radius * 0.8,
                counts_font_size,
            )?;
            let _ = visits.set_attribute("class", "visits");
            let _ = g.append_with_node_1(&visits);

            let _ = self.game_grid.append_with_node_1(&g);

            self.letters.push(Letter {
                group: g,
                starts,
                visits,
            });
        }

        let _ = self.game_grid.set_attribute(
            "viewBox",
            &format!("0 0 {} {}", self.geometry.width, self.geometry.height),
        );

        Ok(())
    }

    fn show_game_contents(&self) {
        let _ = self.context.message.style().set_property("display", "none");
        let _ = self.game_contents.style().set_property("display", "block");
    }

    fn set_element_text(&self, element: &web_sys::HtmlElement, text: &str) {
        while let Some(child) = element.first_child() {
            let _ = element.remove_child(&child);
        }

        let text = self.context.document.create_text_node(text);
        let _ = element.append_with_node_1(&text);
    }

    fn update_title(&self) {
        if let Some(element) = self.context.document.get_element_by_id("title")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let value = format!("WordRoute #{}", 1);
            self.set_element_text(&element, &value);
        }
    }

    fn update_word_route(&self) -> Result<(), String> {
        if let Some(old_route) = self.context.document.get_element_by_id(
            ROUTE_ID,
        ) {
            old_route.remove();
        }

        if let Some((start_x, start_y)) = self.route_start {
            let g = self.create_svg_element("g")?;
            g.set_id(ROUTE_ID);

            let (cx, cy) = self.geometry.convert_coords(start_x, start_y);

            let circle = self.create_svg_element("circle")?;
            let _ = circle.set_attribute(
                "r",
                &(self.geometry.radius * 0.4).to_string());
            let _ = circle.set_attribute("cx", &cx.to_string());
            let _ = circle.set_attribute("cy", &cy.to_string());

            let _ = g.append_with_node_1(&circle);

            if !self.route_steps.is_empty() {
                let (mut x, mut y) = (start_x, start_y);
                let mut path_d = format!("M {},{}", cx, cy);

                for &dir in self.route_steps.iter() {
                    (x, y) = directions::step(x, y, dir);
                    let (x, y) = self.geometry.convert_coords(x, y);
                    write!(&mut path_d, "L {},{}", x, y).unwrap();
                }

                let path = self.create_svg_element("path")?;
                let _ = path.set_attribute("d", &path_d);
                let _ = path.set_attribute(
                    "stroke-width",
                    &(self.geometry.radius * 0.3).to_string(),
                );

                let _ = g.append_with_node_1(&path);
            }

            let _ = self.game_grid.append_with_node_1(&g);
        }

        Ok(())
    }

    fn route_word(&self) -> String {
        let mut word = String::new();

        if let Some((mut x, mut y)) = self.route_start {
            word.push(self.grid.at(x, y));

            for &dir in self.route_steps.iter() {
                (x, y) = directions::step(x, y, dir);
                word.push(self.grid.at(x, y));
            }
        }

        word
    }

    fn try_set_route_word(&mut self, word: &str) -> bool {
        // Hack to work around the borrow checker
        let mut route_steps = std::mem::take(&mut self.route_steps);

        let result;

        if let Some(word_finder::Route { start_x, start_y, steps }) =
            self.word_finder.find(&self.grid, &word)
        {
            route_steps.clear();
            route_steps.extend(steps.into_iter());
            self.route_start = Some((start_x, start_y));

            result = true;
        } else {
            result = false;
        }

        self.route_steps = route_steps;

        result
    }

    fn handle_escape(&mut self) {
        if self.route_start.is_some() {
            self.route_start = None;
            let _ = self.update_word_route();
        }
    }

    fn handle_backspace(&mut self) {
        if self.route_start.is_some() {
            if self.route_steps.pop().is_none() {
                self.route_start = None;
            } else {
                // Removing a character can change the route
                // completely so let’s search for the word again
                let word = self.route_word();
                self.try_set_route_word(&word);
            }

            let _ = self.update_word_route();
        }
    }

    fn handle_letter(&mut self, letter: char) {
        let mut word = self.route_word();

        word.push(letter);

        if self.try_set_route_word(&word) {
            let _ = self.update_word_route();
        }
    }

    fn handle_keydown_event(&mut self, event: web_sys::KeyboardEvent) {
        let key = event.key();

        if key == "Backspace" {
            self.handle_backspace();
        } else if key == "Escape" {
            self.handle_escape();
        } else {
            let mut chars = key.chars();

            if let Some(ch) = chars.next() {
                if chars.next().is_none() {
                    self.handle_letter(ch);
                }
            }
        }
    }
}

fn hexagon_path(radius: f32) -> String {
    let mut result = String::new();

    for i in 0..6 {
        let angle = i as f32 * 2.0 * PI / 6.0;

        write!(
            &mut result,
            "{} {} {} ",
            if i == 0 { 'M' } else { 'L' },
            radius * angle.sin(),
            radius * -angle.cos(),
        ).unwrap();
    }

    result.push('z');

    result
}

fn get_count_value(array: &js_sys::Array, key: u32) -> Result<u8, ()> {
    array.get(key).as_f64().ok_or_else(|| {
        show_error("Error getting count value");
        ()
    }).map(|v| v as u8)
}

fn parse_counts(data: &JsValue, grid: &Grid) -> Result<GridCounts, ()> {
    let Ok(counts_array) = Reflect::get(&data, &"counts".into())
        .map_err(|_| ())
        .and_then(|v| TryInto::<js_sys::Array>::try_into(v).map_err(|_| ()))
    else {
        show_error("Error getting puzzle counts");
        return Err(());
    };

    let mut counts = GridCounts::new(grid.width(), grid.height());

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let starts = get_count_value(
                &counts_array,
                (y * grid.width() + x) * 2,
            )?;
            let visits = get_count_value(
                &counts_array,
                (y * grid.width() + x) * 2 + 1,
            )?;

            *counts.at_mut(x, y) = TileCounts { starts, visits };
        }
    }

    Ok(counts)
}

fn parse_words(data: &JsValue) -> Result<HashMap<String, Word>, ()> {
    let Ok(words_object) = Reflect::get(&data, &"words".into())
        .map_err(|_| ())
        .and_then(|v| TryInto::<js_sys::Object>::try_into(v).map_err(|_| ()))
    else {
        show_error("Error getting word list");
        return Err(());
    };

    let words_array = js_sys::Object::keys(&words_object);

    let mut words = HashMap::new();

    for i in 0..words_array.length() {
        let word_value = words_array.get(i);

        let Some(type_num) = Reflect::get(&words_object, &word_value)
            .ok()
            .and_then(|v| v.as_f64())
        else {
            show_error("Word type is not a float");
            return Err(());
        };

        let Ok(word) = TryInto::<String>::try_into(word_value)
        else {
            show_error("Error getting word from the list");
            return Err(());
        };

        let length = word.chars().count();

        let word_type = if type_num == 0.0 {
            WordType::Normal
        } else if type_num == 1.0 {
            WordType::Bonus
        } else {
            show_error("Unknown word type");
            return Err(());
        };

        words.insert(
            word,
            Word {
                word_type,
                found: false,
                length,
            },
        );
    }

    Ok(words)
}

fn parse_puzzle(data: JsValue) -> Result<Puzzle, ()> {
    let Ok(grid_str) = Reflect::get(&data, &"grid".into())
        .map_err(|_| ())
        .and_then(|v| TryInto::<String>::try_into(v).map_err(|_| ()))
    else {
        show_error("Error getting puzzle grid");
        return Err(())
    };

    let grid = match Grid::new(&grid_str) {
        Ok(g) => g,
        Err(e) => {
            show_error(&e.to_string());
            return Err(());
        },
    };

    let counts = parse_counts(&data, &grid)?;
    let words = parse_words(&data)?;

    Ok(Puzzle {
        grid,
        counts,
        words,
    })
}

fn parse_puzzles(data: JsValue) -> Result<Vec<Puzzle>, ()> {
    let puzzle = parse_puzzle(data)?;

    Ok(vec![puzzle])
}

#[wasm_bindgen]
pub fn init_wordroute() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let context = match Context::new() {
        Ok(c) => c,
        Err(e) => {
            show_error(&e);
            return;
        }
    };

    let loader = Loader::new(context);

    let floating_pointer = loader.start_floating();

    unsafe {
        (*floating_pointer).queue_data_load();
    }
}
