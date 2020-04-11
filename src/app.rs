use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement, HtmlElement};
use yew::{html, Callback, MouseEvent, Component, ComponentLink, Html, ShouldRender, NodeRef};
use yew::services::{IntervalService, RenderService, Task};
use yew::services::console::ConsoleService;
use yew::html::InputData;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use yew::components::Select;

use fnv::FnvHashSet;
use std::time::Duration;
extern crate js_sys;


const DEFAULT_CELL_SIZE: u32 = 5; // px
const CANVAS_MULTIPLIER: u32 = 5;
const GRID_COLOR: &str = "#CCCCCC";
const DEAD_COLOR: &str = "#FFFFFF";
const ALIVE_COLOR: &str = "#FF0000";
const DEFAULT_HEIGHT: u32 = 150;
const DEFAULT_WIDTH: u32 = 150;
const DEFAULT_RENDER_MULTIPLIER: u64 = 1;
const DEFAULT_TICKS_PER_RENDER: u64 = 1;
const MIN_MILLIS_PER_TICK: u64 = 33;
const RANDOM_ALIVE: f64 = 0.2_f64;

pub struct Grid {
    // It would be nice to switch to Morton space
    cells: FnvHashSet<(u32, u32)>,
    active_cells: FnvHashSet<(u32, u32)>,
}

impl Grid {

    pub fn new() -> Grid {
        let cells = FnvHashSet::default();
        let active_cells = FnvHashSet::default();

        Grid {
            cells,
            active_cells
        }
    }

    pub fn randomize_region(&mut self, x_start: u32, y_start: u32, x_end: u32, y_end: u32) {
        let mut row = x_start;

        while row != x_end.wrapping_add(1) {
            let mut col = y_start;
            while col != y_end.wrapping_add(1) {            
                if js_sys::Math::random() < RANDOM_ALIVE {
                    self.cells.insert((row, col));
                    self.update_active(row, col);
                } else {
                    self.cells.remove(&(row, col));
                    self.update_active(row, col);
                }
                col = col.wrapping_add(1);
            }
            row = row.wrapping_add(1);
        }
    }
    
    pub fn update_active(&mut self, row: u32, col: u32) {
        let mut cur_row = row.wrapping_sub(1);
        
        while cur_row != row.wrapping_add(2) {
            let mut cur_col = col.wrapping_sub(1);
            while cur_col != col.wrapping_add(2) {  
                self.active_cells.insert((cur_row, cur_col));
                cur_col = cur_col.wrapping_add(1);
            }
            cur_row = cur_row.wrapping_add(1);
        }
    }

    pub fn cell_alive(&self, height: u32, width: u32) -> bool {
        self.cells.contains(&(height, width))
    }

    pub fn toggle_cell(&mut self, height: u32, width: u32) {
        if !self.cells.remove(&(height, width)) {
            self.cells.insert((height, width));
            self.update_active(height, width);
        } 
    }

    pub fn reset(&mut self) {
        self.cells.clear();
        self.active_cells.clear();
    }

    pub fn step(&mut self) {
        let mut newly_alive: FnvHashSet<(u32, u32)> = FnvHashSet::default();
        let mut newly_dead: FnvHashSet<(u32, u32)> = FnvHashSet::default();

        for &(row, col) in self.active_cells.iter() {

            let num_neighbors = self.num_neighbors(row, col);
            
            let next_alive = match num_neighbors {
                2 if self.cells.contains(&(row, col)) => {
                    true
                },
                3 => {
                    true
                }
                _ => false,
            };

            if next_alive && !self.cells.contains(&(row, col)) {
                newly_alive.insert((row, col));
            } else if !next_alive && self.cells.contains(&(row, col)) {
                newly_dead.insert((row, col));
            }
        }
        self.active_cells.clear();
        for &(row, col) in newly_alive.iter().chain(newly_dead.iter()) {
            self.update_active(row, col)
        }
        let not_killed: FnvHashSet<(u32, u32)> = self.cells.difference(&newly_dead).cloned().collect();
        let next_iter: FnvHashSet<(u32, u32)> = not_killed.union(&newly_alive).cloned().collect();
        self.cells = next_iter;
    }

    pub fn num_neighbors(&self, row: u32, col: u32) -> i32 {

        let mut num_neighbors = 0;

        let mut cur_row = row.wrapping_sub(1);
        
        while cur_row != row.wrapping_add(2) {
            let mut cur_col = col.wrapping_sub(1);
            while cur_col != col.wrapping_add(2) {  
                if (cur_row != row) || (cur_col != col) {
                    if self.cells.contains(&(cur_row, cur_col)) {
                        num_neighbors += 1;
                    }
                }
                cur_col = cur_col.wrapping_add(1);
            }
            cur_row = cur_row.wrapping_add(1);
        }
        num_neighbors
    }
}

pub struct App {
    canvas: Option<HtmlCanvasElement>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    link: ComponentLink<Self>,
    timer: Box<dyn Task>,
    active: bool,
    grid: Grid,
    height: u32,
    width: u32,
    console: ConsoleService,
    render_multiplier: u64,
    steps_per_render: u64,
    moving: bool,
    has_moved: bool,
    cell_offset_x: u32,
    cell_offset_y: u32,
    move_start_x: i32,
    move_start_y: i32,
    cell_size: u32,
}

pub enum Msg {
    Draw,
    Tick,
    Noop,
    PlayPause,
    Random,
    Reset,
    Step,
    ResizeWidth(InputData),
    ResizeHeight(InputData),
    Slower,
    Faster,
    StartMove(MouseEvent),
    Move(MouseEvent),
    EndMove(MouseEvent),
    Smaller,
    Bigger,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();
    
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut interval = IntervalService::new();
        let handle = interval.spawn(Duration::from_millis(DEFAULT_RENDER_MULTIPLIER * MIN_MILLIS_PER_TICK), link.callback(|_| Msg::Tick));

        let width = DEFAULT_WIDTH;
        let height = DEFAULT_HEIGHT;

        App {
            canvas: None,
            link: link,
            node_ref: NodeRef::default(),
            render_loop: None,
            timer: Box::new(handle),
            active: false,
            grid: Grid::new(),
            height,
            width,
            console: ConsoleService::new(),
            render_multiplier: DEFAULT_RENDER_MULTIPLIER,
            steps_per_render: DEFAULT_TICKS_PER_RENDER,
            moving: false,
            has_moved: false,
            cell_offset_x: 0,
            cell_offset_y: 0,
            move_start_x: 0,
            move_start_y: 0,
            cell_size: DEFAULT_CELL_SIZE,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        // Once mounted, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas: HtmlCanvasElement = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        
        canvas.set_width(self.width * CANVAS_MULTIPLIER);
        canvas.set_height(self.width * CANVAS_MULTIPLIER);

        self.canvas = Some(canvas);
        let render_frame = self.link.callback(|_| Msg::Draw);
        let handle = RenderService::new().request_animation_frame(render_frame);

        // A reference to the handle must be stored, otherwise it is dropped and the render won't
        // occur.
        self.render_loop = Some(Box::new(handle));
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::PlayPause => {
                self.active = !self.active;
                true
            }
            Msg::Draw => {
                self.draw_cells();
                false
            }
            Msg::Tick => {
                if self.active && !self.moving {
                    
                    for _step in 0..self.steps_per_render {
                        self.grid.step();
                    }
                    self.draw_cells();
                }
                false
            },
            Msg::Random => {
                self.randomize_region();
                self.draw_cells();
                false
            }
            Msg::Noop => {
                false
            },
            Msg::Reset => {
                self.grid.reset();
                self.resize();
                false
            },
            Msg::Step => {
                self.grid.step();                
                self.draw_cells();
                false
            },
            Msg::ResizeWidth(event) => {
                if let Ok(width) = event.value.parse::<u32>() {
                    self.width = width;
                    self.resize();
                    //self.grid = Grid::new(self.width, self.height);
                } else {
                    self.console.log(&format!("Invalid width: {}", event.value));
                }
                false
            },
            Msg::ResizeHeight(event) => {
                if let Ok(height) = event.value.parse::<u32>() {
                    self.height = height;
                    self.resize();
                    //self.grid = Grid::new(self.width, self.height);
                } else {
                    self.console.log(&format!("Invalid height: {}", event.value));
                }
                false
            },
            Msg::Slower => {
                if self.steps_per_render > 1 {
                    self.steps_per_render -= 1;
                } else {
                    self.render_multiplier += 1;
                    let mut interval = IntervalService::new();
                    let handle = interval.spawn(Duration::from_millis(self.render_multiplier * MIN_MILLIS_PER_TICK), self.link.callback(|_| Msg::Tick));
                    self.timer = Box::new(handle);            
                }
                self.console.log(&format!("Steps per render: {} Render multiplier: {}", self.steps_per_render, self.render_multiplier));
                false
            },
            Msg::Faster => {
                if self.render_multiplier > 1 {
                    self.render_multiplier -= 1;
                    let mut interval = IntervalService::new();
                    let handle = interval.spawn(Duration::from_millis(self.render_multiplier * MIN_MILLIS_PER_TICK), self.link.callback(|_| Msg::Tick));
                    self.timer = Box::new(handle);  
                } else {
                    self.steps_per_render += 1;
                }
                self.console.log(&format!("Steps per render: {} Render multiplier: {}", self.steps_per_render, self.render_multiplier));
                false
            },
            Msg::StartMove(event) => {                
                self.move_start_x = event.client_x();
                self.move_start_y = event.client_y();
                self.moving = true;
                false
            },
            Msg::EndMove(event) => {
                if !self.has_moved {
                    self.toggle_cell(event);
                    self.draw_cells();                        
                }
                self.moving = false;
                self.has_moved = false;
                false
            },
            Msg::Move(event) => {
                if !self.moving {
                    return false;
                }

                let (starting_cell_x, starting_cell_y) = self.page_coordinates_to_cell_coordinates(self.move_start_x, self.move_start_y);
                let (current_cell_x, current_cell_y) = self.page_coordinates_to_cell_coordinates(event.client_x(), event.client_y());
                
                if starting_cell_x != current_cell_x || starting_cell_y != current_cell_y {
                    self.console.log(&format!("cur x: {} start x: {} cur y: {} start y: {}", event.client_x(), self.move_start_x, event.client_y(), self.move_start_y));
                    self.cell_offset_x = self.cell_offset_x.wrapping_add(starting_cell_x).wrapping_sub(current_cell_x);
                    self.cell_offset_y = self.cell_offset_y.wrapping_add(starting_cell_y).wrapping_sub(current_cell_y);
                    self.move_start_x = event.client_x();
                    self.move_start_y = event.client_y();
                    self.console.log(&format!("starting cell x: {} current cell x: {} starting cell y: {} current cell y: {}", starting_cell_x, current_cell_x, starting_cell_y, current_cell_y));
                    self.console.log(&format!("cell offset x: {} cell offset y: {}", self.cell_offset_x, self.cell_offset_y));
                    self.draw_cells();
                    self.has_moved = true;
                }
                false
            },
            Msg::Smaller => {
                self.cell_size = self.cell_size.saturating_sub(1);
                self.draw_cells();
                false
            },
            Msg::Bigger => {
                self.cell_size += 1;
                self.draw_cells();
                false
            }
        }
    }

    fn view(&self) -> Html {
        let play_or_pause_text = if self.active {
            "Pause"
        } else {
            "Play"
        };

        html! {
            <body                 onmouseup=self.link.callback(|event| Msg::EndMove(event))>
                <div class="game-buttons">
                    <button class="game-button" onclick=self.link.callback(|_| Msg::PlayPause)> { play_or_pause_text }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Random)>{ "Random" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Step)>{ "Step" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Reset)>{ "Reset" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Slower)>{ "Slower" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Faster)>{ "Faster" }</button>
                    /*<button class="game-button" onclick=self.link.callback(|_| Msg::Smaller)>{ "Smaller" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Bigger)>{ "Bigger" }</button>
                    <label for="width">{ "Width"}</label>
                    <input class="game-text" oninput=self.link.callback(|event| Msg::ResizeWidth(event)) placeholder=DEFAULT_WIDTH type="number" id="width" name="width" min=1/>
                    <label for="height">{ "Height"}</label>
                    <input class="game-text" oninput=self.link.callback(|event| Msg::ResizeHeight(event)) placeholder=DEFAULT_HEIGHT type="number" id="height" name="height" min=1/>
                    */
                </div>
                <div>
                    <canvas ref={self.node_ref.clone()} style="border: 1px solid black;"
                     onmousedown=self.link.callback(|event| Msg::StartMove(event))
                     
                     onmousemove=self.link.callback(|event| Msg::Move(event))>
                    { "This text is displayed if your browser does not support HTML5 Canvas." }
                    </canvas>                    
                </div>
            </body>
        }
    }
}

impl App {

    fn randomize_region(&mut self) {
        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        let canvas_width = canvas.width() as f64;
        let canvas_height = canvas.height() as f64;

        let (start_x, start_y) = self.canvas_coordinates_to_cell_coordinates(0_f64, 0_f64);
        let (end_x, end_y) = self.canvas_coordinates_to_cell_coordinates(canvas_width, canvas_height);
        self.grid.randomize_region(start_x, start_y, end_x, end_y);
    }

    fn page_coordinates_to_canvas_coordinates(&self, page_x: i32, page_y: i32) -> (f64, f64) {
        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        let canvas_width = canvas.width() as f64;
        let canvas_height = canvas.height() as f64;

        let rect = canvas.dyn_into::<Element>()
            .expect("cant coerce into element")
            .get_bounding_client_rect();

        let scale_x = canvas_width / rect.width();
        let scale_y = canvas_height / rect.height();

        let canvas_left = (page_x as f64 - rect.left()) * scale_x;
        let canvas_top = (page_y as f64 - rect.top()) * scale_y;

        (canvas_left, canvas_top)
    }

    fn canvas_coordinates_to_cell_coordinates(&self, canvas_x: f64, canvas_y: f64) -> (u32, u32) {
        let unadjusted_row = (canvas_y / self.cell_size as f64) as u32;
        let unadjusted_col = (canvas_x / self.cell_size as f64) as u32;

        self.adjust_coords(unadjusted_row, unadjusted_col)
    }

    fn adjust_coords(&self, unadjusted_row: u32, unadjusted_col: u32) -> (u32, u32) {
        let row = unadjusted_row.wrapping_add(self.cell_offset_x);
        let col = unadjusted_col.wrapping_add(self.cell_offset_y);
        (row, col)
    }

    fn page_coordinates_to_cell_coordinates(&self, page_x: i32, page_y: i32) -> (u32, u32) {
        let (canvas_x, canvas_y) = self.page_coordinates_to_canvas_coordinates(page_x, page_y);
        self.canvas_coordinates_to_cell_coordinates(canvas_x, canvas_y)
    }

    fn toggle_cell(&mut self, event: MouseEvent) {        
        let (row, col) = self.page_coordinates_to_cell_coordinates(event.client_x(), event.client_y());
        self.grid.toggle_cell(row, col);
    }

    fn draw_cells(&mut self) {

        let ctx = self.canvas.as_ref()
            .expect("Canvas not loaded")
            .get_context("2d")
            .expect("Can't get 2d canvas.")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
                
        ctx.begin_path();
        ctx.set_fill_style(&JsValue::from_str(DEAD_COLOR));
        ctx.fill_rect(0_f64, 0_f64, (self.width * CANVAS_MULTIPLIER) as f64, (self.height * CANVAS_MULTIPLIER) as f64);
        
        ctx.begin_path();
        ctx.set_fill_style(&JsValue::from_str(ALIVE_COLOR));

        for unadjusted_row in 0..self.width {
            for unadjusted_col in 0..self.height {
                let (row_idx, col_idx) = self.adjust_coords(unadjusted_row, unadjusted_col);
                let cell_alive = self.grid.cell_alive(row_idx, col_idx);                

                if cell_alive {
                    ctx.rect(unadjusted_col as f64 * (self.cell_size as f64) ,
                    unadjusted_row as f64* (self.cell_size as f64),
                    self.cell_size as f64,
                    self.cell_size as f64);
                }
            }
        }
        ctx.fill();
        let render_frame = self.link.callback(|_| Msg::Noop);
        let handle = RenderService::new().request_animation_frame(render_frame);
        self.render_loop = Some(Box::new(handle));
    }

    fn resize(&mut self) {
        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        canvas.set_width(self.width * CANVAS_MULTIPLIER);
        canvas.set_height(self.height * CANVAS_MULTIPLIER);

        self.canvas = Some(canvas);
        let render_frame = self.link.callback(|_| Msg::Draw);
        let handle = RenderService::new().request_animation_frame(render_frame);

        // A reference to the handle must be stored, otherwise it is dropped and the render won't
        // occur.
        self.render_loop = Some(Box::new(handle));
    }
}