use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement, HtmlElement};
use yew::{html, Callback, MouseEvent, Component, ComponentLink, Html, ShouldRender, NodeRef};
use yew::services::{IntervalService, RenderService, Task};
use yew::services::console::ConsoleService;
use yew::html::InputData;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use yew::components::Select;

use std::time::Duration;
extern crate js_sys;


const CELL_SIZE: u32 = 5; // px
const GRID_COLOR: &str = "#CCCCCC";
const DEAD_COLOR: &str = "#FFFFFF";
const ALIVE_COLOR: &str = "#FF0000";
const DEFAULT_HEIGHT: u32 = 150;
const DEFAULT_WIDTH: u32 = 150;
const DEFAULT_RENDER_MULTIPLIER: u64 = 10;
const DEFAULT_TICKS_PER_RENDER: u64 = 1;
const MIN_MILLIS_PER_TICK: u64 = 33;
const RANDOM_ALIVE: f64 = 0.2_f64;


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cell {
    Alive,
    Dead,
}

pub struct Grid {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
    previous_cells: Vec<Cell>,
}

impl Grid {

    pub fn new(width: u32, height: u32) -> Grid {
        let num_cells = (width * height) as usize;
        let cells = vec![Cell::Dead; num_cells];
        let previous_cells = vec![Cell::Dead; num_cells];

        Grid {
            width,
            height,
            cells,
            previous_cells
        }
    }

    pub fn randomize(&mut self) {
        for idx in 0..self.cells.len() {
            if js_sys::Math::random() < RANDOM_ALIVE {
                self.cells[idx] = Cell::Alive;
            } else {
                self.cells[idx] = Cell::Dead;
            }
        }
    }

    pub fn get_idx(&self, height: u32, width: u32) -> usize {
        ((height * self.width) + width) as usize
    }

    pub fn get_cell(&self, height: u32, width: u32) -> Cell {
        let idx = self.get_idx(height, width);
        self.cells[idx]
    }

    pub fn get_previous_cell(&self, height: u32, width: u32) -> Cell {
        let idx = self.get_idx(height, width);
        self.previous_cells[idx]
    }

    pub fn set_cell(&mut self, height: u32, width: u32, cell: Cell) {
        let idx = self.get_idx(height, width);
        self.cells[idx] = cell;
    }

    pub fn toggle_cell(&mut self, height: u32, width: u32) {
        let idx = self.get_idx(height, width);
        let cell = self.cells[idx];

        if cell == Cell:: Alive {
            self.cells[idx] = Cell::Dead;
        } else {
            self.cells[idx] = Cell::Alive;
        }
    }

    pub fn reset(&mut self) {
        for idx in 0..self.cells.len() {
            self.cells[idx] = Cell::Dead;
        }
    }

    pub fn step(&mut self) {

        for i in 0..self.height {
            for j in 0..self.width {

                let num_neighbors = self.num_neighbors(i, j);
                let idx = self.get_idx(i, j);
                match num_neighbors {
                    x if x < 2 => {self.previous_cells[idx] = Cell::Dead},
                    2 => {self.previous_cells[idx] = self.cells[idx];},
                    3 => {self.previous_cells[idx] = Cell::Alive;},
                    _ => {self.previous_cells[idx] = Cell::Dead;},
                }
            }
        }
        std::mem::swap(&mut self.cells, &mut self.previous_cells);
    }

    pub fn num_neighbors(&self, height: u32, width: u32) -> i32 {

        let mut num_neighbors = 0;

        for x_offset in 0..3 {
            for y_offset in 0..3 {
                
                if x_offset == 1 && y_offset == 1 {
                    continue;
                }

                let wrapped_x = (height + self.height + x_offset - 1) % self.height;
                let wrapped_y = (width + self.width + y_offset - 1) % self.width;

                if self.get_cell(wrapped_x, wrapped_y) == Cell::Alive {
                    num_neighbors += 1;
                }
            }
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
}

pub enum Msg {
    Draw,
    Tick,
    Noop,
    PlayPause,
    Random,
    Reset,
    Step,
    Toggle(MouseEvent),
    ResizeWidth(InputData),
    ResizeHeight(InputData),
    Slower,
    Faster,
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
            grid: Grid::new(500, 500),
            height,
            width,
            console: ConsoleService::new(),
            render_multiplier: DEFAULT_RENDER_MULTIPLIER,
            steps_per_render: DEFAULT_TICKS_PER_RENDER,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        // Once mounted, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas: HtmlCanvasElement = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        
        canvas.set_width(self.width * (CELL_SIZE) + 1);
        canvas.set_height(self.width * (CELL_SIZE) + 1);

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
                if self.active {
                    
                    for _step in 0..self.steps_per_render {
                        self.grid.step();
                    }
                    self.draw_cells();
                }
                false
            },
            Msg::Random => {
                self.grid.randomize();
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
            Msg::Toggle(event) => {
                self.toggle_cell(event);
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
                    self.steps_per_render /= 2;
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
                    self.steps_per_render *= 2;
                }
                self.console.log(&format!("Steps per render: {} Render multiplier: {}", self.steps_per_render, self.render_multiplier));
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
            <body>
                <div class="game-buttons">
                    <button class="game-button" onclick=self.link.callback(|_| Msg::PlayPause)> { play_or_pause_text }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Random)>{ "Random" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Step)>{ "Step" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Reset)>{ "Reset" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Slower)>{ "Slower" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Faster)>{ "Faster" }</button>
                    <label for="width">{ "Width"}</label>
                    <input class="game-text" oninput=self.link.callback(|event| Msg::ResizeWidth(event)) placeholder=DEFAULT_WIDTH type="number" id="width" name="width" min=1/>
                    <label for="height">{ "Height"}</label>
                    <input class="game-text" oninput=self.link.callback(|event| Msg::ResizeHeight(event)) placeholder=DEFAULT_HEIGHT type="number" id="height" name="height" min=1/>
                </div>
                <div>
                    <canvas ref={self.node_ref.clone()} onclick=self.link.callback(|event| Msg::Toggle(event)) style="border: 1px solid black;"/>                    
                </div>
            </body>
        }
    }
}

impl App {

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
        let row = ((canvas_y / (CELL_SIZE as f64 )) as u32).min(self.height - 1);
        let col = ((canvas_x / (CELL_SIZE as f64 )) as u32).min(self.width - 1);
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
        ctx.set_fill_style(&JsValue::from_str(ALIVE_COLOR));

        for row_idx in 0..self.height {
            for col_idx in 0..self.width {
                let current = self.grid.get_cell(row_idx, col_idx);
                let previous = self.grid.get_previous_cell(row_idx, col_idx);

                if current == Cell::Alive {
                    ctx.rect(col_idx as f64 * (CELL_SIZE as f64) ,
                    row_idx as f64* (CELL_SIZE as f64) ,
                    CELL_SIZE as f64,
                    CELL_SIZE as f64);
                } 
            }
        }
        ctx.fill();
        ctx.begin_path();
        ctx.set_fill_style(&JsValue::from_str(DEAD_COLOR));
        for row_idx in 0..self.height {
            for col_idx in 0..self.width { 
                let current = self.grid.get_cell(row_idx, col_idx);
                let previous = self.grid.get_previous_cell(row_idx, col_idx);

                if current == Cell::Dead {
                    ctx.rect(col_idx as f64 * (CELL_SIZE as f64) ,
                    row_idx as f64* (CELL_SIZE as f64),
                    CELL_SIZE as f64,
                    CELL_SIZE as f64);
                }
            }
        }

        ctx.fill();;
        let render_frame = self.link.callback(|_| Msg::Noop);
        let handle = RenderService::new().request_animation_frame(render_frame);
        self.render_loop = Some(Box::new(handle));
    }

    fn resize(&mut self) {
        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        canvas.set_width(self.width * (CELL_SIZE) + 1);
        canvas.set_height(self.height * (CELL_SIZE) + 1);

        self.canvas = Some(canvas);
        let render_frame = self.link.callback(|_| Msg::Draw);
        let handle = RenderService::new().request_animation_frame(render_frame);

        // A reference to the handle must be stored, otherwise it is dropped and the render won't
        // occur.
        self.render_loop = Some(Box::new(handle));
    }
}