use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::services::{IntervalService, RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};

use fnv::FnvHashMap;
use std::time::Duration;
use std::collections::hash_map::Iter;

/* A coordinate system for the Packed Cells.
 1 point here represents 64 cells
 u32 in two dimensions is basically an infinite universe
 Smallest dimension is ~2 billion in size
 It wraps at 0, u32::max_value() + 1 so there are no weird edge effects
*/
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PackedCoordinates(u32, u32);

// Represents one 1x64 row of cells
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PackedCells(u64);

impl std::fmt::Debug for PackedCells {

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

/*
Lookup from a 3x3 region to a 1x1 region (1 bit)

e.g. in Conway's Game of Life:
1 0 1
0 0 0 -> 1
1 0 0

There are 2**9 = 512 distinct 3x3 regions

Bits are arranged as:
1 2 3
4 5 6
7 8 9
*/
pub struct StepTable(Box<[u8]>);

pub struct PackedCellMap(FnvHashMap<PackedCoordinates, PackedCells>);

impl PackedCoordinates {

    pub fn get_x(&self) -> u32 {
        self.0
    }

    pub fn get_y(&self) -> u32 {
        self.1
    }
}

impl StepTable {

    pub fn new(b: Vec<u32>, s: Vec<u32>) -> StepTable {
        StepTable(StepTable::make_lookup(b, s))
    }

    pub fn make_lookup(b: Vec<u32>, s: Vec<u32>) -> Box<[u8]> {
        let mut lookup = Vec::with_capacity(512);

        for idx in 0..512 {
            let target_cell_masked = idx & 0b111101111 as u32;
            let num_neighbors = target_cell_masked.count_ones();
            let alive = b.contains(&num_neighbors) || (s.contains(&num_neighbors) && (idx & 0b10000 == 1));
            if alive {
                lookup.push(1_u8);
            } else {
                lookup.push(0_u8);
            }
            println!("{:b}, {:b} {}", idx, target_cell_masked, alive);
        }
        lookup.into_boxed_slice()
    }

    pub fn step(
        &self,
        stagger_right: bool, 
        r1: PackedCells, r2: PackedCells,
        r3: PackedCells, r4: PackedCells,
        r5: PackedCells, r6: PackedCells
    ) -> PackedCells {

        if stagger_right {            
            let mut new_cells = 0_u64;

            for offset in 0..=61 {
                let region = ((r1.0 & (0b111 << offset)) >> offset) // top 1x3 row
                    | ((r1.0 & (0b111 << offset)) >> offset << 3) // middle 1x3 row
                    | ((r2.0 & (0b111 << offset)) >> offset << 6); // bottom 1x3 row
                new_cells |= (self.lookup(region) as u64) << offset;
            }
            
            let rightmost = (r1.0 >> 63) | ((r4.0 & 0b11) << 1) // top 1x3 row
                | ((r2.0 >> 63) << 3) | ((r5.0 & 0b11) << 4) // middle 1x3 row
                | ((r3.0 >> 63) << 6) | ((r6.0 & 0b11) << 7); // bottom 1x3 row
            new_cells |= (self.lookup(rightmost) as u64) << 62;

            let second_rightmost = (r1.0 >> 62) | ((r4.0 & 0b1) << 2)
                | ((r2.0 >> 62) << 3) | ((r5.0 & 0b1) << 5)
                | ((r3.0 >> 62) << 6) | ((r6.0 & 0b1) << 8);
                
            new_cells |= (self.lookup(second_rightmost) as u64) << 63;

            PackedCells(new_cells)
        } else {
            PackedCells(0)
        }        
    }

    pub fn lookup(&self, region: u64) -> u8 {
        self.0[region as usize]
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

        let start = PackedCoordinates(0_u32, 0_u32);
        let cells = PackedCells(0b11101101101011111);
        self.universe.add(start, cells);
        self.universe.add(PackedCoordinates(0_u32, 3_u32), PackedCells(!0b11101101101011111));

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
            let mut cells = packed_cells.0 as u64;
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
    fn test_step() {
        let r1 = PackedCells(0b1);
        let r4 = PackedCells(0); 
        let lookup = StepTable::new(vec![3], vec![2]);
        //println!("{:?}", lookup.0);
        let expected = PackedCells(0b1);

        assert_eq!(expected, lookup.step(true, r1, r1, r1, r4, r4, r4));

        let expected = PackedCells(0x8000000000000000);
        assert_eq!(expected, lookup.step(true, r4, r4, r4, r1, r1, r1));
    }

    /*#[test]
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

    #[test]
    fn test_new_block_forward() {
        let top_left = PackedCells{top: 0b1111, bottom: 0b11110000};
        let bottom_left = PackedCells{top: 0b10, bottom: 0};
        let top_right = PackedCells{top:0b10, bottom: 0};
        let bottom_right = PackedCells{top: 0b01 << 30, bottom: 0};

        let expected_r1 = Block::concat_rows(true, top_left.top, top_right.top);
        let expected_r2 = Block::concat_rows(true, top_left.bottom, top_right.bottom);
        let expected_r3 = Block::concat_rows(true, bottom_left.top, bottom_right.top);
        let expected_r4 = Block::concat_rows(true, bottom_left.bottom, bottom_right.bottom);
        let expected = Block(expected_r1, expected_r2, expected_r3, expected_r4);
        assert_eq!(expected, Block::new(true, top_left, bottom_left, top_right, bottom_right));
    }*/
}