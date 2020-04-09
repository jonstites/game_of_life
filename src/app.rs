use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};
use yew::{html, Callback, MouseEvent, Component, ComponentLink, Html, ShouldRender, NodeRef};
use yew::services::{IntervalService, RenderService, Task};
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;

use std::time::Duration;
extern crate js_sys;


const CELL_SIZE: u32 = 5; // px
const GRID_COLOR: &str = "#CCCCCC";
const DEAD_COLOR: &str = "#FFFFFF";
const ALIVE_COLOR: &str = "#FF0000";
const HEIGHT: u32 = 640;
const WIDTH: u32 = 640;
const MILLIS_PER_TICK: u64 = 50;
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
        ((height * WIDTH) + width) as usize
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
}

impl Component for App {
    type Message = Msg;
    type Properties = ();
    
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut interval = IntervalService::new();
        let handle = interval.spawn(Duration::from_millis(MILLIS_PER_TICK), link.callback(|_| Msg::Tick));

        App {
            canvas: None,
            link: link,
            node_ref: NodeRef::default(),
            render_loop: None,
            timer: Box::new(handle),
            active: false,
            grid: Grid::new(WIDTH, HEIGHT),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        // Once mounted, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas: HtmlCanvasElement = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        canvas.set_width(WIDTH * (CELL_SIZE + 1) + 1);
        canvas.set_height(WIDTH * (CELL_SIZE + 1) + 1);

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
                self.draw_grid();
                self.draw_cells();
                false
            }
            Msg::Tick => {
                if self.active {
                    self.grid.step();
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
                self.draw_cells();
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
            }
        }
    }

    fn view(&self) -> Html {
        let play_pause = if self.active {
            "Pause"
        } else {
            "Play"
        };

        html! {
            <body>
                <div class="game-buttons">
                    <button class="game-button" onclick=self.link.callback(|_| Msg::PlayPause)> { play_pause }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Random)>{ "Random" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Step)>{ "Step" }</button>
                    <button class="game-button" onclick=self.link.callback(|_| Msg::Reset)>{ "Reset" }</button>
                </div>
                <div>
                    <canvas ref={self.node_ref.clone()} onclick=self.link.callback(|event| Msg::Toggle(event))/>
                </div>
            </body>
        }
    }
}

impl App {

    fn toggle_cell(&mut self, event: MouseEvent) {
        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        let canvas_width = canvas.width();
        let canvas_height = canvas.height();

        let rect = canvas.dyn_into::<Element>()
            .expect("cant coerce into element")
            .get_bounding_client_rect();

        let scale_x = canvas_width as f64 / rect.width();
        let scale_y = canvas_height as f64 / rect.height();
      
        let canvas_left = (event.client_x() as f64 - rect.left()) * scale_y;
        let canvas_top = (event.client_y() as f64 - rect.top()) * scale_y;
      
        let row = ((canvas_top / (CELL_SIZE as f64 + 1_f64)) as u32).min(HEIGHT - 1);
        let col = ((canvas_left / (CELL_SIZE as f64 + 1_f64)) as u32).min(WIDTH - 1);

        self.grid.toggle_cell(row, col);
    }

    fn draw_grid(&mut self) {
        let ctx = self.canvas.as_ref()
            .expect("Canvas not loaded")
            .get_context("2d")
            .expect("Can't get 2d canvas.")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        ctx.begin_path();
        ctx.set_stroke_style(&JsValue::from_str(GRID_COLOR));
    
        // Vertical lines.
        for i in 0..=WIDTH {
            let x_value = i as f64 * (CELL_SIZE as f64 + 1_f64) + 1_f64;
            let y_start = 0_f64;
            let y_end = (CELL_SIZE as f64 + 1_f64) * HEIGHT as f64 + 10_f64;

            ctx.move_to(x_value, y_start);
            ctx.line_to(x_value, y_end);
        }
        
        // Horizontal lines.
        for j in 0..=HEIGHT {
            let y_value = j as f64 * (CELL_SIZE as f64 + 1_f64) + 1_f64;
            let x_start = 0_f64;
            let x_end = (CELL_SIZE as f64 + 1_f64) * WIDTH as f64 + 10_f64;

            ctx.move_to(x_start, y_value);
            ctx.line_to(x_end, y_value);
        }
    
        ctx.stroke();
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
        for row_idx in 0..HEIGHT {
            for col_idx in 0..WIDTH {
                let current = self.grid.get_cell(row_idx, col_idx);
                let previous = self.grid.get_previous_cell(row_idx, col_idx);

                if current != previous && current == Cell::Alive {
                    ctx.fill_rect(col_idx as f64 * (CELL_SIZE as f64 + 1_f64) + 1_f64,
                    row_idx as f64* (CELL_SIZE as f64 + 1_f64) + 1_f64,
                    CELL_SIZE as f64,
                    CELL_SIZE as f64);
                } 
            }
        }        
        ctx.set_fill_style(&JsValue::from_str(DEAD_COLOR));
        for row_idx in 0..HEIGHT {
            for col_idx in 0..WIDTH { 
                let current = self.grid.get_cell(row_idx, col_idx);
                let previous = self.grid.get_previous_cell(row_idx, col_idx);

                if current != previous && current == Cell::Dead {
                    ctx.fill_rect(col_idx as f64 * (CELL_SIZE as f64 + 1_f64) + 1_f64,
                    row_idx as f64* (CELL_SIZE as f64 + 1_f64) + 1_f64,
                    CELL_SIZE as f64,
                    CELL_SIZE as f64);
                }
            }
        }

        ctx.stroke();
        let render_frame = self.link.callback(|_| Msg::Noop);
        let handle = RenderService::new().request_animation_frame(render_frame);
        self.render_loop = Some(Box::new(handle));
    }
}