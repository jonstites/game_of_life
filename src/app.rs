use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::services::{IntervalService, RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};

use fnv::FnvHashMap;
use std::time::Duration;
use std::collections::hash_map::Iter;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PackedCoordinates(u16, u16);
pub struct PackedCells{
    top: u32,
    bottom: u32,
}

#[derive(PartialEq, Eq)]
pub struct BlockRow(u64);
#[derive(Debug, PartialEq, Eq)]
pub struct Block(BlockRow, BlockRow, BlockRow, BlockRow);
pub struct PackedCellMap(FnvHashMap<PackedCoordinates, PackedCells>);

impl std::fmt::Debug for BlockRow {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:b}", self.0)        
    }
}
impl PackedCoordinates {

    pub fn get_x(&self) -> u16 {
        self.0
    }

    pub fn get_y(&self) -> u16 {
        self.1
    }
}

impl Block {

    pub fn new(
        stagger_forward: bool, 
        top_left: PackedCells, 
        bottom_left: PackedCells, 
        top_right: PackedCells, 
        bottom_right: PackedCells) -> Block {

        let row_1 = Block::concat_rows(stagger_forward, top_left.top, top_right.top);
        let row_2 = Block::concat_rows(stagger_forward, top_left.bottom, top_right.bottom);
        let row_3 = Block::concat_rows(stagger_forward, bottom_left.top, bottom_right.top);
        let row_4 = Block::concat_rows(stagger_forward, bottom_left.bottom, bottom_right.bottom);
        Block(row_1, row_2, row_3, row_4)
    }

    pub fn concat_rows(stagger_forward: bool, left: u32, right: u32) -> BlockRow {
        let left = left as u64;
        let right = right as u64;

        if stagger_forward {
            BlockRow(left | ((right & 0b11) << 32))
        } else {
            BlockRow(((left & (0b11 << 30)) >> 30) | (right << 2))
        }
    }
}

/*
Packed Cells contain 2 32-cell rows (row major format)

00 01 02 03 04 05 06 07 ... 1f
20 21 22 23 24 25 26 27 ... 3f

Four Packed Cells are used to create a new 2x32 Packed Cells for the
next generation.

This is done by reading a 4x4 region at a time and determining the
next generation of the 2x2 inner region.
*/
pub struct Universe {
    cells: PackedCellMap,
    stagger_forward: bool,
}

impl Universe {
    pub fn new() -> Universe {
        Universe {
            cells: PackedCellMap(FnvHashMap::default()),
            stagger_forward: true,
        }
    }

    pub fn add(&mut self, coordinates: PackedCoordinates, cells: PackedCells) {
        self.cells.0.insert(coordinates, cells);
    }
    
    pub fn get_packed_cells(&self) -> Iter<PackedCoordinates, PackedCells> {
        self.cells.0.iter()
    }


}

pub struct App {
    canvas: Option<HtmlCanvasElement>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    link: ComponentLink<Self>,
    #[allow(dead_code)]
    timer: Box<dyn Task>,
    universe: Universe,
}

pub enum Msg {
    Draw,
    Tick,
    Noop,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut interval = IntervalService::new();
        let handle = interval.spawn(Duration::from_millis(1000), link.callback(|_| Msg::Tick));

        App {
            canvas: None,
            link,
            node_ref: NodeRef::default(),
            render_loop: None,
            timer: Box::new(handle),
            universe: Universe::new(),
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        // Once mounted, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas: HtmlCanvasElement = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        canvas.set_width(200);
        canvas.set_height(200);

        let start = PackedCoordinates(0_u16, 0_u16);
        let cells = PackedCells{top:0b11101101101011111, bottom: 0};
        self.universe.add(start, cells);
        self.universe.add(PackedCoordinates(0_u16, 3_u16), PackedCells{top:!0b11101101101011111, bottom: 0});

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
            Msg::Draw => {
                self.draw_cells();
                false
            }
            Msg::Tick => {
                self.universe.stagger_forward = !self.universe.stagger_forward;
                self.draw_cells();
                false
            },
            Msg::Noop => false,
        }
    }

    fn view(&self) -> Html {
        html! {
            <body>
                <div>
                    <canvas ref={self.node_ref.clone()} style="border: 1px solid black;">
                    { "This text is displayed if your browser does not support HTML5 Canvas." }
                    </canvas>
                </div>
            </body>
        }
    }
}

impl App {
    fn draw_cells(&mut self) {
        let ctx = self
            .canvas
            .as_ref()
            .expect("Canvas not loaded")
            .get_context("2d")
            .expect("Can't get 2d canvas.")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        let cell_size = 10;

        
        ctx.set_fill_style(&JsValue::from_str("white"));

        ctx.fill_rect(0_f64, 0_f64, 500_f64, 500_f64);

        ctx.begin_path();
        ctx.set_fill_style(&JsValue::from_str("red"));


        for (coordinates, packed_cells) in self.universe.get_packed_cells() {
            let mut num = 0;
            let mut cells = packed_cells.top as u64;
            while cells > 0 {
                if cells & 1_u64 == 1_u64 {
                    let mut x_start = ((coordinates.get_x() * 64 + num) * cell_size) as f64;
                    let mut y_start = (coordinates.get_y() * cell_size) as f64;
                    if !self.universe.stagger_forward {
                        x_start -= cell_size as f64;
                        y_start -= cell_size as f64;
                    }

                    ctx.rect(
                        x_start,
                        y_start,
                        cell_size as f64,
                        cell_size as f64,
                    );
                }
                cells >>= 1;
                num += 1;
            }
        }
        ctx.fill();

        let render_frame = self.link.callback(|_| Msg::Noop);
        let handle = RenderService::new().request_animation_frame(render_frame);
        self.render_loop = Some(Box::new(handle));
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_rows_concat() {
        let left = u32::max_value();
        let right = 0b10;

        let expected = BlockRow(0b10_1111_1111_1111_1111_1111_1111_1111_1111);
        assert_eq!(expected, Block::concat_rows(true, left, right));

        let expected = BlockRow(0b1011);
        assert_eq!(expected, Block::concat_rows(false, left, right));

        let expected = 33;
        assert_eq!(expected, Block::concat_rows(true, left, right).0.count_ones());
        let expected = 3;
        assert_eq!(expected, Block::concat_rows(false, left, right).0.count_ones());

    }

    /*#[test]
    fn test_new_block_forward() {
        let top_left = PackedCells{top: 0b1111, bottom: 0b11110000};
        let bottom_left = PackedCells{top: 0b10, bottom: }
        let top_right = PackedCells(0b10 << 30);
        let bottom_right = PackedCells(0b01 << 30);

        let expected = Block(0b111110, 0b100, 0b1111000001, 0);
        assert_eq!(expected, Block::new(true, top_left, bottom_left, top_right, bottom_right));
    }*/
}