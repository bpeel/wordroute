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
use super::puzzle::{Puzzle, WordType, N_HINT_LEVELS};
use super::save_state::{self, SaveState};
use std::fmt::Write;
use js_sys::Reflect;
use std::f32::consts::PI;
use std::collections::{hash_map, HashMap};

const SAVE_STATE_KEY: &'static str = "wordroute-save-states";
const SVG_NAMESPACE: &'static str = "http://www.w3.org/2000/svg";
const ROUTE_ID: &'static str = "route-line";
const SORT_HINT_CHECKBOX_ID: &'static str = "sort-hint-checkbox";
const LETTERS_HINT_CHECKBOX_ID: &'static str = "letters-hint-checkbox";
const SHARE_TEXT_COPIED_ID: &'static str = "share-text-copied";
const SHARE_TEXT_ID: &'static str = "share-text";

const STARTS_HINT_LEVEL: usize = 1;
const VISITS_HINT_LEVEL: usize = 2;
const WORDS_HINT_LEVEL: usize = 3;

#[derive(PartialEq, Eq, Copy, Clone)]
enum Page {
    Game,
    Instructions,
    Share,
}

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
        let filename = "puzzles.json";

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

    fn start_game(&mut self, puzzles: Vec<PuzzleData>) {
        let Loader { context, .. } = self.stop_floating();

        if let Some(puzzle_num) = get_chosen_puzzle(&context) {
            match Wordroute::new(context, puzzles, puzzle_num) {
                Ok(wordroute) => {
                    // Leak the main wordroute object so that it will live as
                    // long as the web page
                    std::mem::forget(wordroute);
                },
                Err(e) => show_error(&e.to_string()),
            }
        } else {
            build_puzzle_list(&context, puzzles);
        }
    }
}

struct Letter {
    group: web_sys::SvgElement,
    starts: web_sys::SvgElement,
    visits: web_sys::SvgElement,
}

struct PuzzleData {
    grid: Grid,
    counts: GridCounts,
    words: Vec<(String, WordType)>,
}

struct Wordroute {
    context: Context,
    pointerdown_closure: Option<Closure::<dyn Fn(JsValue)>>,
    pointerup_closure: Option<Closure::<dyn Fn(JsValue)>>,
    pointermove_closure: Option<Closure::<dyn Fn(JsValue)>>,
    pointercancel_closure: Option<Closure::<dyn Fn(JsValue)>>,
    keydown_closure: Option<Closure::<dyn Fn(JsValue)>>,
    hints_changed_closure: Option<Closure::<dyn Fn(JsValue)>>,
    visibility_closure: Option<Closure::<dyn Fn(JsValue)>>,
    close_closure: Option<Closure::<dyn Fn(JsValue)>>,
    help_closure: Option<Closure::<dyn Fn(JsValue)>>,
    share_closure: Option<Closure::<dyn Fn(JsValue)>>,
    copy_closure: Option<Closure::<dyn Fn(JsValue)>>,
    game_contents: web_sys::HtmlElement,
    word_count: web_sys::HtmlElement,
    score_bar: web_sys::HtmlElement,
    current_word: web_sys::HtmlElement,
    word_message: web_sys::HtmlElement,
    game_grid: web_sys::SvgElement,
    puzzle: Puzzle,
    chosen_puzzle: usize,
    letters: Vec<Option<Letter>>,
    geometry: Geometry,
    word_finder: word_finder::Finder,
    word: String,
    route_start: Option<(u32, u32)>,
    route_steps: Vec<u8>,
    try_route_buf: Vec<u8>,
    pointer_tail: Option<(u32, u32)>,
    word_lists: HashMap<usize, web_sys::HtmlElement>,
    sort_word_lists: bool,
    show_some_letters: bool,
}

impl Wordroute {
    fn new(
        context: Context,
        puzzles: Vec<PuzzleData>,
        chosen_puzzle: usize,
    ) -> Result<Box<Wordroute>, String> {
        let Some(game_contents) =
            context.document.get_element_by_id("game-contents")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get game contents".to_string());
        };

        let Some(word_count) =
            context.document.get_element_by_id("word-count")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get current-word".to_string());
        };

        let Some(score_bar) =
            context.document.get_element_by_id("score-bar")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get current-word".to_string());
        };

        let Some(current_word) =
            context.document.get_element_by_id("current-word")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get current-word".to_string());
        };

        let Some(word_message) =
            context.document.get_element_by_id("word-message")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get word-message".to_string());
        };

        let Some(game_grid) = context.document.get_element_by_id("game-grid")
            .and_then(|c| c.dyn_into::<web_sys::SvgElement>().ok())
        else {
            return Err("failed to get game grid".to_string());
        };

        let Some(PuzzleData { grid, counts, words }) = puzzles
            .into_iter()
            .nth(chosen_puzzle.wrapping_sub(1))
        else {
            return Err("chosen puzzle is not available".to_string());
        };

        let geometry = Geometry::new(&grid, 100.0);

        let puzzle = Puzzle::new(grid, counts, words);

        let mut wordroute = Box::new(Wordroute {
            context,
            pointerdown_closure: None,
            pointerup_closure: None,
            pointermove_closure: None,
            pointercancel_closure: None,
            keydown_closure: None,
            hints_changed_closure: None,
            visibility_closure: None,
            close_closure: None,
            help_closure: None,
            share_closure: None,
            copy_closure: None,
            game_contents,
            word_count,
            score_bar,
            current_word,
            word_message,
            game_grid,
            puzzle,
            chosen_puzzle,
            geometry,
            letters: Vec::new(),
            word_finder: word_finder::Finder::new(),
            word: String::new(),
            route_start: None,
            route_steps: Vec::new(),
            try_route_buf: Vec::new(),
            pointer_tail: None,
            word_lists: HashMap::new(),
            sort_word_lists: false,
            show_some_letters: false,
        });

        wordroute.create_letters()?;

        wordroute.create_closures();
        wordroute.set_up_close_button();
        wordroute.set_up_help_button();
        wordroute.set_up_share_button();
        wordroute.set_up_copy_button();
        wordroute.update_title(chosen_puzzle);
        wordroute.create_word_lists()?;

        let save_states = load_save_states(&wordroute.context);

        if let Some(save_state) = save_states.get(&chosen_puzzle) {
            wordroute.puzzle.load_save_state(&save_state);
        }

        wordroute.flush_puzzle_changes();

        wordroute.remove_loading_class();
        wordroute.show_game_contents();

        if save_states.is_empty() {
            wordroute.set_page(Page::Instructions);
        }

        Ok(wordroute)
    }

    fn save_to_local_storage(&mut self) {
        let Some(save_state) = self.puzzle.changed_save_state()
        else {
            return;
        };

        let Some(local_storage) = get_local_storage(&self.context)
        else {
            return;
        };

        let mut save_states = load_save_states_from_local_storage(
            &local_storage
        );

        save_states.insert(self.chosen_puzzle, save_state);

        let mut save_states_string = String::new();

        save_state::serialize_multiple(
            &mut save_states_string,
            &save_states,
        ).unwrap();

        if let Err(_) =
            local_storage.set_item(SAVE_STATE_KEY, &save_states_string)
        {
            console::log_1(&"Error saving state".into());
        }
    }

    fn create_closures(&mut self) {
        let wordroute_pointer = self as *mut Wordroute;

        let pointerdown_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                wordroute.handle_pointerdown_event(event);
            }
        );

        let _ = self.game_grid.add_event_listener_with_callback(
            "pointerdown",
            pointerdown_closure.as_ref().unchecked_ref(),
        );

        self.pointerdown_closure = Some(pointerdown_closure);

        let pointerup_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                wordroute.handle_pointerup_event(event);
            }
        );

        let _ = self.game_grid.add_event_listener_with_callback(
            "pointerup",
            pointerup_closure.as_ref().unchecked_ref(),
        );

        self.pointerup_closure = Some(pointerup_closure);

        let pointermove_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                wordroute.handle_pointermove_event(event);
            }
        );

        let _ = self.game_grid.add_event_listener_with_callback(
            "pointermove",
            pointermove_closure.as_ref().unchecked_ref(),
        );

        self.pointermove_closure = Some(pointermove_closure);

        let pointercancel_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                wordroute.handle_pointercancel_event(event);
            }
        );

        let _ = self.game_grid.add_event_listener_with_callback(
            "pointercancel",
            pointercancel_closure.as_ref().unchecked_ref(),
        );

        self.pointercancel_closure = Some(pointercancel_closure);

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

        let hints_changed_closure = Closure::<dyn Fn(JsValue)>::new(
            move |_event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                wordroute.handle_hints_changed();
            }
        );

        for id in [SORT_HINT_CHECKBOX_ID, LETTERS_HINT_CHECKBOX_ID].iter() {
            if let Some(element) = self.context.document.get_element_by_id(id) {
                let _ = element.add_event_listener_with_callback(
                    "change",
                    hints_changed_closure.as_ref().unchecked_ref(),
                );
            }
        }

        self.hints_changed_closure = Some(hints_changed_closure);

        let visibility_closure = Closure::<dyn Fn(JsValue)>::new(
            move |_event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                wordroute.save_to_local_storage();
            }
        );

        let _ = self.context.document.add_event_listener_with_callback(
            "visibilitychange",
            visibility_closure.as_ref().unchecked_ref(),
        );

        self.visibility_closure = Some(visibility_closure);
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
        y: f32,
        font_size: f32,
    ) -> Result<web_sys::SvgElement, String> {
        let elem = self.create_svg_element("text")?;
        let _ = elem.set_attribute("text-anchor", "middle");
        let _ = elem.set_attribute("x", "0");
        let _ = elem.set_attribute("y", &y.to_string());
        let _ = elem.set_attribute("font-size", &font_size.to_string());

        Ok(elem)
    }

    fn set_up_close_button(&mut self) {
        let wordroute_pointer = self as *mut Wordroute;

        let close_closure = Closure::<dyn Fn(JsValue)>::new(
            move |_event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                wordroute.set_page(Page::Game);
            }
        );

        for id in [
            "close-instructions",
            "close-instructions-cross",
            "close-share-cross",
        ] {
            let Some(close_button) =
                self.context.document.get_element_by_id(id)
                .and_then(|c| c.dyn_into::<web_sys::EventTarget>().ok())
            else {
                continue;
            };

            let _ = close_button.add_event_listener_with_callback(
                "click",
                close_closure.as_ref().unchecked_ref(),
            );
        }

        self.close_closure = Some(close_closure);
    }

    fn set_up_help_button(&mut self) {
        let wordroute_pointer = self as *mut Wordroute;

        let help_closure = Closure::<dyn Fn(JsValue)>::new(
            move |_event: JsValue| {
                let wordroute = unsafe { &mut *wordroute_pointer };
                wordroute.set_page(Page::Instructions);
            }
        );

        let Some(help_button) =
            self.context.document.get_element_by_id("help-button")
            .and_then(|c| c.dyn_into::<web_sys::EventTarget>().ok())
        else {
            return;
        };

        let _ = help_button.add_event_listener_with_callback(
            "click",
            help_closure.as_ref().unchecked_ref(),
        );

        self.help_closure = Some(help_closure);
    }

    fn set_up_share_button(&mut self) {
        let wordroute_pointer = self as *mut Wordroute;

        let share_closure = Closure::<dyn Fn(JsValue)>::new(
            move |_event: JsValue| {
                let wordroute = unsafe { &*wordroute_pointer };
                wordroute.show_share_page();
            }
        );

        let Some(share_button) =
            self.context.document.get_element_by_id("share-button")
            .and_then(|c| c.dyn_into::<web_sys::EventTarget>().ok())
        else {
            return;
        };

        let _ = share_button.add_event_listener_with_callback(
            "click",
            share_closure.as_ref().unchecked_ref(),
        );

        self.share_closure = Some(share_closure);
    }

    fn set_up_copy_button(&mut self) {
        let wordroute_pointer = self as *mut Wordroute;

        let copy_closure = Closure::<dyn Fn(JsValue)>::new(
            move |_event: JsValue| {
                let wordroute = unsafe { &*wordroute_pointer };
                wordroute.copy_share_text();
            }
        );

        let Some(copy_button) =
            self.context.document.get_element_by_id("copy-share")
            .and_then(|c| c.dyn_into::<web_sys::EventTarget>().ok())
        else {
            return;
        };

        let _ = copy_button.add_event_listener_with_callback(
            "click",
            copy_closure.as_ref().unchecked_ref(),
        );

        self.copy_closure = Some(copy_closure);
    }

    fn show_share_page(&self) {
        if let Some(share_text_elem) =
            self.context.document.get_element_by_id(SHARE_TEXT_ID)
        {
            let mut share_text = self.puzzle.share_text(self.chosen_puzzle);

            if let Some(url) = self.context.document.location()
                .and_then(|location| location.href().ok())
            {
                share_text.push('\n');
                share_text.push_str(&url);
            }

            set_element_text(&share_text_elem, &share_text);

            self.set_element_visibility(SHARE_TEXT_COPIED_ID, false);
        }

        self.set_page(Page::Share);
    }

    fn copy_share_text(&self) {
        let Some(share_text_elem) =
            self.context.document.get_element_by_id(SHARE_TEXT_ID)
            .and_then(|c| c.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        else {
            console::log_1(&"Error getting share text element".into());
            return;
        };

        share_text_elem.select();

        let copy_result = self.context.document.exec_command("copy");

        let _ = share_text_elem.set_selection_range(0, 0);

        if copy_result.is_err() {
            console::log_1(&"copy command failed".into());
        } else {
            self.set_element_visibility(SHARE_TEXT_COPIED_ID, true);
        }
    }

    fn create_letters(&mut self) -> Result<(), String> {
        let hexagon_path = hexagon_path(self.geometry.radius);

        let font_size = self.geometry.radius;
        let text_y_pos = self.geometry.radius * 0.25;

        let counts_font_size = self.geometry.radius * 0.3;

        for (x, y) in (0..self.puzzle.height())
            .map(|y| (0..self.puzzle.width()).map(move |x| (x, y)))
            .flatten()
        {
            let letter = self.puzzle.grid().at(x, y);

            if letter == '.' {
                self.letters.push(None);
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

            let text = self.create_letter_text(text_y_pos, font_size)?;

            set_element_text(&text, &self.puzzle.grid().at(x, y).to_string());

            let _ = g.append_with_node_1(&text);

            let starts = self.create_letter_text(
                -self.geometry.radius * 0.6,
                counts_font_size,
            )?;
            let _ = starts.set_attribute("class", "starts");
            let _ = g.append_with_node_1(&starts);

            let visits = self.create_letter_text(
                self.geometry.radius * 0.8,
                counts_font_size,
            )?;
            let _ = visits.set_attribute("class", "visits");
            let _ = g.append_with_node_1(&visits);

            let _ = self.game_grid.append_with_node_1(&g);

            self.letters.push(Some(Letter {
                group: g,
                starts,
                visits,
            }));
        }

        let _ = self.game_grid.set_attribute(
            "viewBox",
            &format!("0 0 {} {}", self.geometry.width, self.geometry.height),
        );

        Ok(())
    }

    fn create_word_lists(&mut self) -> Result<(), String> {
        let Some(word_lists_element) =
            self.context.document.get_element_by_id("word-lists")
        else {
            return Err("failed to get word-lists element".to_string());
        };

        for length in self.puzzle.word_lists().into_iter() {
            if let hash_map::Entry::Vacant(entry) =
                self.word_lists.entry(length)
            {
                let Ok(title) = self.context.document.create_element("h2")
                else {
                    return Err("error creating title".to_string());
                };

                set_element_text(&title, &format!("{} letters", length));

                let _ = word_lists_element.append_with_node_1(&title);

                let Some(div) = self.context.document.create_element("div").ok()
                    .and_then(|d| d.dyn_into::<web_sys::HtmlElement>().ok())
                else {
                    return Err("error creating div".to_string());
                };

                let _ = word_lists_element.append_with_node_1(&div);

                entry.insert(div);
            }
        }

        Ok(())
    }

    fn update_word_list_for_length(&self, length: usize) {
        let Some(div) = self.word_lists.get(&length)
        else {
            return;
        };

        clear_element(div);

        let Ok(list_div) = self.context.document.create_element("div")
        else {
            return;
        };

        let _ = div.append_with_node_1(&list_div);

        let mut missing_word_count = 0;
        let mut found_words = Vec::new();

        for (key, word) in self.puzzle.words() {
            if word.length != length || word.word_type != WordType::Normal {
                continue;
            }

            if word.found || self.show_some_letters || self.sort_word_lists {
                found_words.push((key, word.found));
            } else {
                missing_word_count += 1;
            }
        }

        found_words.sort_unstable_by_key(|&(word, found)| {
            (!found && !self.sort_word_lists, word)
        });

        let width = format!("{}em", length as f32 * 0.9);

        let (start_letters, end_letters);

        if self.show_some_letters {
            start_letters = length.saturating_sub(2) / 2;
            end_letters = length.saturating_sub(3) / 4;
        } else {
            start_letters = 0;
            end_letters = 0;
        };

        let mut text_buf = String::new();

        for &(word, found) in found_words.iter() {
            let Some(span) = self.context.document.create_element("span").ok()
                .and_then(|d| d.dyn_into::<web_sys::HtmlElement>().ok())
            else {
                continue;
            };

            let _ = span.style().set_property("width", &width);

            if found {
                set_element_text(&span, word);
            } else {
                let mut chars = word.chars();

                text_buf.clear();

                for _ in 0..start_letters {
                    text_buf.push(chars.next().unwrap());
                }

                for _ in 0..(length - start_letters - end_letters) {
                    text_buf.push('*');
                    let _ = chars.next();
                }

                for _ in 0..end_letters {
                    text_buf.push(chars.next().unwrap());
                }

                set_element_text(&span, &text_buf);
            }

            let _ = list_div.append_with_node_1(&span);
        }

        if missing_word_count > 0 {
            if let Ok(missing_div) =
                self.context.document.create_element("div")
            {
                if missing_word_count == 1 {
                    set_element_text(&missing_div, "+1 word left");
                } else {
                    set_element_text(
                        &missing_div,
                        &format!("+{} words left", missing_word_count),
                    );
                }

                let _ = div.append_with_node_1(&missing_div);
            }
        }
    }

    fn update_all_word_lists(&self) {
        for &length in self.word_lists.keys() {
            self.update_word_list_for_length(length);
        }
    }

    fn show_game_contents(&self) {
        let _ = self.context.message.style().set_property("display", "none");
        let _ = self.game_contents.style().set_property("display", "block");
    }

    fn update_title(&self, chosen_puzzle: usize) {
        if let Some(element) = self.context.document.get_element_by_id("title")
        {
            let value = format!("WordRoute #{}", chosen_puzzle);
            set_element_text(&element, &value);
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

    fn update_word(&self) {
        let _ = self.update_word_route();

        self.current_word.set_text_content(Some(&self.word));
    }

    fn try_route_word(&mut self) -> bool {
        self.try_route_buf.clear();

        if let Some(start) =
            self.word_finder.find(
                self.puzzle.grid(),
                &self.word,
                &mut self.try_route_buf,
            )
        {
            std::mem::swap(&mut self.route_steps, &mut self.try_route_buf);
            self.route_start = Some(start);

            true
        } else {
            false
        }
    }

    fn clear_word(&mut self) {
        self.route_start = None;
        self.word.clear();
    }

    fn animate_word_message(&self) {
        // Re-add the element to trigger the animation
        if let Some(parent) = self.word_message.parent_node() {
            self.word_message.remove();
            let _ = parent.append_child(&self.word_message);
        }
    }

    fn set_element_visibility(&self, id: &str, visibility: bool) {
        if let Some(elem) =
            self.context.document.get_element_by_id(id)
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = elem.style().set_property(
                "display",
                if visibility { "block" } else { "none" },
            );
        }
    }

    fn set_page(&self, page: Page) {
        self.set_element_visibility("content", page == Page::Game);
        self.set_element_visibility(
            "instructions-overlay",
            page == Page::Instructions,
        );
        self.set_element_visibility("share-overlay", page == Page::Share);
    }

    fn remove_loading_class(&self) {
        if let Some(content) =
            self.context.document.get_element_by_id("content")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = content.class_list().remove_1("loading");
        }
    }

    fn flush_puzzle_changes(&mut self) {
        if let Some(n_words_found) = self.puzzle.changed_n_words_found() {
            set_element_text(
                &self.word_count,
                &format!(
                    "{} / {} words",
                    n_words_found,
                    self.puzzle.total_n_words(),
                ),
            );
        }

        if let Some(n_letters_found) = self.puzzle.changed_n_letters_found() {
            let _ = self.score_bar.style().set_property(
                "width",
                &format!(
                    "{}%",
                    n_letters_found * 100 / self.puzzle.total_n_letters()),
            );
        }

        if let Some(hint_level) = self.puzzle.changed_hint_level() {
            self.update_hint_level(hint_level);
        }

        for (x, y) in self.puzzle.changed_counts() {
            self.update_counts_text(x, y);
        }

        if let Some(message) = self.puzzle.pending_word_message() {
            set_element_text(&self.word_message, message);
            self.animate_word_message();
        }

        for length in self.puzzle.changed_word_lists() {
            self.update_word_list_for_length(length);
        }
    }

    fn set_hint_style(&self, style: &str, value: bool) {
        let class_list = self.game_contents.class_list();

        if value {
            let _ = class_list.add_1(style);
        } else {
            let _ = class_list.remove_1(style);
        }
    }

    fn update_next_level_marker(&self, hint_level: usize) {
        let Some(marker) = self.context.document.get_element_by_id(
            "next-level-marker"
        ).and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return;
        };

        let _ = marker.style().set_property(
            "display",
            if hint_level + 1 < N_HINT_LEVELS {
                "block"
            } else {
                "none"
            },
        );

        let mut marker_text = String::new();
        let left_anchor = hint_level + 1 <= N_HINT_LEVELS / 2;

        if left_anchor {
            marker_text.push_str("⇤ ");
        }
        marker_text.push_str("next hint");
        if !left_anchor {
            marker_text.push_str(" ⇥");
        }

        set_element_text(&marker, &marker_text);

        if left_anchor {
            let _ = marker.style().set_property(
                "left",
                &format!("{}%", (hint_level + 1) * 100 / N_HINT_LEVELS),
            );
            let _ = marker.style().remove_property("right");
        } else {
            let _ = marker.style().set_property(
                "right",
                &format!(
                    "{}%",
                    100 - (hint_level + 1) * 100 / N_HINT_LEVELS
                ),
            );
            let _ = marker.style().remove_property("left");
        }
    }

    fn update_hint_level(&mut self, hint_level: usize) {
        self.set_hint_style(
            "no-starts-hint",
            hint_level < STARTS_HINT_LEVEL,
        );
        self.set_hint_style(
            "no-visits-hint",
            hint_level < VISITS_HINT_LEVEL,
        );
        self.set_hint_style(
            "no-words-hint",
            hint_level < WORDS_HINT_LEVEL,
        );

        self.update_next_level_marker(hint_level);
    }

    fn update_counts_text(&self, x: u32, y: u32) {
        let counts = self.puzzle.counts().at(x, y);

        if let Some(letter) = &self.letters[
            ((y * self.puzzle.width()) + x) as usize
        ] {
            set_element_text(&letter.starts, &counts.starts.to_string());
            set_element_text(&letter.visits, &counts.visits.to_string());

            if counts.visits <= 0 {
                let _ = letter.group.class_list().add_1("finished");
            }
        }
    }

    fn send_word(&mut self) {
        self.puzzle.score_word(&self.word);
        self.clear_word();
        let _ = self.update_word_route();
        self.flush_puzzle_changes();
    }

    fn position_for_event(
        &self,
        event: &web_sys::PointerEvent,
    ) -> Option<(u32, u32)> {
        let Some(target) = event.target()
        else {
            return None;
        };

        let Ok(element) = target.dyn_into::<web_sys::SvgElement>()
        else {
            return None;
        };

        if element != self.game_grid {
            return None;
        }

        let pointer_x = event.offset_x();
        let pointer_y = event.offset_y();
        let client_width = element.client_width();

        // Convert the pointer coordinates to the viewBox space of the
        // game grid
        let grid_x = pointer_x as f32 * 100.0 / client_width as f32;
        let grid_y = pointer_y as f32 * 100.0 / client_width as f32;

        let (tile_x, tile_y) = self.geometry.reverse_coords(grid_x, grid_y);

        if tile_x >= self.puzzle.width() ||
            tile_y >= self.puzzle.height() ||
            self.puzzle.grid().at(tile_x as u32, tile_y as u32) == '.'
        {
            None
        } else {
            Some((tile_x as u32, tile_y as u32))
        }
    }

    fn get_checkbox_value(&self, checkbox_id: &str) -> bool {
        self.context.document.get_element_by_id(checkbox_id)
            .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|c| c.checked())
            .unwrap_or(false)
    }

    fn handle_escape(&mut self) {
        if self.route_start.is_some() && self.pointer_tail.is_none() {
            self.clear_word();
            let _ = self.update_word();
        }
    }

    fn handle_backspace(&mut self) {
        if self.route_start.is_some() && self.pointer_tail.is_none() {
            self.word.pop().unwrap();

            if self.route_steps.pop().is_none() {
                self.route_start = None;
            } else {
                // Removing a character can change the route
                // completely so let’s search for the word again
                let try_result = self.try_route_word();
                assert!(try_result);
            }

            let _ = self.update_word();
        }
    }

    fn handle_enter(&mut self) {
        if self.pointer_tail.is_some() {
            return;
        }

        self.send_word();
    }

    fn handle_letter(&mut self, letter: char) {
        if self.pointer_tail.is_some() {
            return;
        }

        self.word.push(letter);

        if self.try_route_word() {
            let _ = self.update_word();
        } else {
            self.word.pop();
        }
    }

    fn handle_pointerdown_event(&mut self, event: web_sys::PointerEvent) {
        if !event.is_primary() || event.button() != 0 {
            return;
        }

        event.prevent_default();

        let Some(position) = self.position_for_event(&event)
        else {
            return;
        };

        let _ = self.game_grid.set_pointer_capture(event.pointer_id());

        self.pointer_tail = Some(position);
        self.route_start = Some(position);
        self.route_steps.clear();
        self.word.clear();
        self.word.push(self.puzzle.grid().at(position.0, position.1));
        let _ = self.update_word();
    }

    fn handle_pointerup_event(&mut self, event: web_sys::PointerEvent) {
        if !event.is_primary() || event.button() != 0 {
            return;
        }

        event.prevent_default();

        if self.pointer_tail.take().is_none() {
            return;
        }

        let _ = self.game_grid.release_pointer_capture(event.pointer_id());

        if self.position_for_event(&event).is_none() {
            if !self.route_steps.is_empty() {
                set_element_text(&self.word_message, "Cancel");
                self.animate_word_message();
            }
            self.clear_word();
            self.update_word();
        } else {
            self.send_word();
        }
    }

    fn handle_pointermove_event(&mut self, event: web_sys::PointerEvent) {
        if !event.is_primary() {
            return;
        }

        event.prevent_default();

        let Some((last_x, last_y)) = self.pointer_tail
        else {
            return;
        };

        let Some((start_x, start_y)) = self.route_start
        else {
            return;
        };

        let Some(position) = self.position_for_event(&event)
        else {
            return;
        };

        // If we’re moving back a space then undo the last move
        if Some(position) == self.route_steps.last().map(|&dir| {
            directions::reverse(last_x, last_y, dir)
        }) {
            self.route_steps.pop().unwrap();
            self.word.pop().unwrap();
            self.pointer_tail = Some(position);
            let _ = self.update_word();
        } else {
            // Can we get here from the previous position?
            let dir = 'find_direction: {
                for dir in 0..directions::N_DIRECTIONS {
                    if position == directions::step(last_x, last_y, dir) {
                        break 'find_direction dir;
                    }
                }

                return;
            };

            // Have we already visited this space?
            let mut x = start_x;
            let mut y = start_y;

            for &dir in self.route_steps.iter() {
                if (x, y) == position {
                    return;
                }

                (x, y) = directions::step(x, y, dir);
            }

            self.route_steps.push(dir);
            self.word.push(self.puzzle.grid().at(position.0, position.1));
            self.pointer_tail = Some(position);
            let _ = self.update_word();
        }
    }

    fn handle_pointercancel_event(&mut self, event: web_sys::PointerEvent) {
        if !event.is_primary() {
            return;
        }

        if self.pointer_tail.is_some() {
            self.pointer_tail = None;
            self.clear_word();
            self.update_word();
        }
    }

    fn handle_keydown_event(&mut self, event: web_sys::KeyboardEvent) {
        let key = event.key();

        if key == "Backspace" {
            self.handle_backspace();
        } else if key == "Escape" {
            self.handle_escape();
        } else if key == "Enter" {
            self.handle_enter();
        } else {
            let mut chars = key.chars();

            if let Some(ch) = chars.next() {
                if chars.next().is_none() {
                    self.handle_letter(ch);
                }
            }
        }
    }

    fn handle_hints_changed(&mut self) {
        self.sort_word_lists = self.get_checkbox_value(SORT_HINT_CHECKBOX_ID);
        self.show_some_letters =
            self.get_checkbox_value(LETTERS_HINT_CHECKBOX_ID);

        if self.sort_word_lists || self.show_some_letters {
            self.puzzle.use_hints();
        }

        self.update_all_word_lists();
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

fn parse_words(data: &JsValue) -> Result<Vec<(String, WordType)>, ()> {
    let Ok(words_object) = Reflect::get(&data, &"words".into())
        .map_err(|_| ())
        .and_then(|v| TryInto::<js_sys::Object>::try_into(v).map_err(|_| ()))
    else {
        show_error("Error getting word list");
        return Err(());
    };

    let words_array = js_sys::Object::keys(&words_object);

    let mut words = Vec::new();

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

        let word_type = if type_num == 0.0 {
            WordType::Normal
        } else if type_num == 1.0 {
            WordType::Bonus
        } else {
            show_error("Unknown word type");
            return Err(());
        };

        words.push((word, word_type));
    }

    Ok(words)
}

fn parse_puzzle(data: JsValue) -> Result<PuzzleData, ()> {
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

    Ok(PuzzleData {
        grid,
        counts,
        words,
    })
}

fn parse_puzzles(data: JsValue) -> Result<Vec<PuzzleData>, ()> {
    let Ok(puzzle_array) = TryInto::<js_sys::Array>::try_into(data)
    else {
        show_error("Error getting puzzle array");
        return Err(());
    };

    let mut puzzles = Vec::new();

    for data in puzzle_array.iter() {
        puzzles.push(parse_puzzle(data)?);
    }

    Ok(puzzles)
}

fn clear_element(element: &web_sys::Element) {
    while let Some(child) = element.first_child() {
        let _ = element.remove_child(&child);
    }
}

fn set_element_text(element: &web_sys::Element, text: &str) {
    clear_element(element);

    if let Some(document) = element.owner_document() {
        let text = document.create_text_node(text);
        let _ = element.append_with_node_1(&text);
    }
}

fn get_chosen_puzzle(context: &Context) -> Option<usize> {
    let location = context.document.location()?;
    let search = location.search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let puzzle_jsvalue = params.get("p")?;
    let puzzle_str: String = puzzle_jsvalue.try_into().ok()?;

    puzzle_str.parse::<usize>().ok()
}

fn build_puzzle_list(context: &Context, puzzles: Vec<PuzzleData>) {
    let Some(puzzle_list) = context.document.get_element_by_id("puzzle-list")
    else {
        show_error("Error getting puzzle list");
        return;
    };

    let Some(path_name) = context.document.location()
        .and_then(|location| location.pathname().ok())
    else {
        show_error("Error getting location path name");
        return;
    };

    for (puzzle_num, puzzle) in puzzles.into_iter().enumerate() {
        let Ok(li) = context.document.create_element("li")
        else {
            continue;
        };

        let Ok(a) = context.document.create_element("a")
        else {
            continue;
        };

        set_element_text(&a, &format!("Puzzle {}", puzzle_num + 1));

        let _ = a.set_attribute(
            "href",
            &format!("{}?p={}", path_name, puzzle_num + 1),
        );

        let _ = li.append_with_node_1(&a);

        let detail = context.document.create_text_node(
            &format!(
                " – {} words",
                puzzle.words.iter()
                    .filter(|&&(_, word_type)| word_type == WordType::Normal)
                    .count(),
            ),
        );

        let _ = li.append_with_node_1(&detail);

        let _ = puzzle_list.append_with_node_1(&li);
    }

    let _ = context.message.style().set_property("display", "none");

    if let Some(puzzle_selector) = context.document.get_element_by_id(
        "puzzle-selector",
    ).and_then(|ps| ps.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = puzzle_selector.style().set_property("display", "block");
    };
}

fn load_save_states_from_local_storage(
    local_storage: &web_sys::Storage,
) -> HashMap<usize, SaveState> {
    match local_storage.get_item(SAVE_STATE_KEY) {
        Ok(Some(save_states)) => {
            match save_state::parse_multiple(&save_states) {
                Ok(save_states) => save_states,
                Err(e) => {
                    console::log_1(&format!(
                        "Error parsing save states: {}",
                        e,
                    ).into());
                    HashMap::new()
                },
            }
        },
        Ok(None) => HashMap::new(),
        Err(_) => {
            console::log_1(&"Error getting save states".into());
            HashMap::new()
        },
    }
}

fn get_local_storage(context: &Context) -> Option<web_sys::Storage> {
    match context.window.local_storage() {
        Ok(Some(local_storage)) => Some(local_storage),
        Ok(None) => {
            console::log_1(&"Local storage is None".into());
            None
        },
        Err(_) => {
            console::log_1(&"Error getting local storage".into());
            None
        },
    }
}

fn load_save_states(context: &Context) -> HashMap<usize, SaveState> {
    if let Some(local_storage) = get_local_storage(context) {
        load_save_states_from_local_storage(&local_storage)
    } else {
        HashMap::new()
    }
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
