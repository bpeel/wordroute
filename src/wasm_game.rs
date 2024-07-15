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
use super::grid_math::Geometry;
use std::fmt::Write;
use js_sys::Reflect;
use std::f32::consts::PI;

const SVG_NAMESPACE: &'static str = "http://www.w3.org/2000/svg";

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

    fn parse_puzzles(&mut self, data: JsValue) -> Result<Vec<Grid>, ()> {
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

        Ok(vec![grid])
    }

    fn data_loaded(&mut self, data: JsValue) {
        match self.parse_puzzles(data) {
            Err(_) => {
                self.stop_floating();
            },
            Ok(puzzles) => self.start_game(puzzles),
        }
    }

    fn start_game(&mut self, puzzles: Vec<Grid>) {
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

struct Wordroute {
    context: Context,
    game_contents: web_sys::HtmlElement,
    game_grid: web_sys::SvgElement,
    letters: Vec<web_sys::SvgElement>,
    grid: Grid,
}

impl Wordroute {
    fn new(
        context: Context,
        puzzles: Vec<Grid>
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

        let Some(grid) = puzzles.into_iter().next()
        else {
            return Err("no puzzles available".to_string());
        };

        let mut wordroute = Box::new(Wordroute {
            context,
            game_contents,
            game_grid,
            grid,
            letters: Vec::new(),
        });

        wordroute.update_title();
        wordroute.create_letters()?;

        wordroute.show_game_contents();

        Ok(wordroute)
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

    fn create_letters(&mut self) -> Result<(), String> {
        let geometry = Geometry::new(&self.grid, 100.0);

        let hexagon_path = hexagon_path(geometry.radius);

        let font_size = format!("{}", geometry.radius * 1.2);
        let text_y_pos = format!("{}", geometry.radius * 0.3);

        for (x, y) in (0..self.grid.height())
            .map(|y| (0..self.grid.width()).map(move |x| (x, y)))
            .flatten()
        {
            let letter = self.grid.at(x, y);

            if letter == '.' {
                continue;
            }

            let g = self.create_svg_element("g")?;

            let x_off = if y & 1 == 0 {
                0.0
            } else {
                geometry.step_x / 2.0
            };

            let _ = g.set_attribute("class", "letter");
            let _ = g.set_attribute(
                "transform",
                &format!(
                    "translate({}, {})",
                    geometry.top_x + x as f32 * geometry.step_x + x_off,
                    geometry.top_y + y as f32 * geometry.step_y,
                ),
            );
            g.set_id(&format!("letter-{}-{}", x, y));

            let path = self.create_svg_element("path")?;
            let _ = path.set_attribute("d", &hexagon_path);

            let _ = g.append_with_node_1(&path);

            let text = self.create_svg_element("text")?;
            let _ = text.set_attribute("text-anchor", "middle");
            let _ = text.set_attribute("x", "0");
            let _ = text.set_attribute("y", &text_y_pos);
            let _ = text.set_attribute("font-size", &font_size);

            let text_node = self.context.document.create_text_node(
                &format!("{}", self.grid.at(x, y)),
            );
            let _ = text.append_with_node_1(&text_node);

            let _ = g.append_with_node_1(&text);

            let _ = self.game_grid.append_with_node_1(&g);

            self.letters.push(g);
        }

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
