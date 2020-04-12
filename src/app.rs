use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::services::{ConsoleService, IntervalService, RenderService, Task};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackedCoordinates(u32, u32);

// Represents one 1x64 row of cells
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct PackedCells(u64);

impl std::fmt::Debug for PackedCells {

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl std::fmt::Display for PackedCells {

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

impl std::ops::Add for PackedCoordinates {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0), self.1.wrapping_add(other.1))
    }
}

impl std::ops::Sub for PackedCoordinates {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0), self.1.wrapping_sub(other.1))
    }
}

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
            let target_cell_masked = idx & 0b111_101_111 as u32;
            let num_neighbors = target_cell_masked.count_ones();
            let alive = b.contains(&num_neighbors) || (s.contains(&num_neighbors) && (idx & 0b000_010_000 != 0));
            if alive {
                lookup.push(1_u8);
            } else {
                lookup.push(0_u8);
            }
            println!("{:b}, {:b} {} {}", idx, target_cell_masked, alive, num_neighbors);
        }
        lookup.into_boxed_slice()
    }

    pub fn step(
        &self,
        staggerred_right: bool, 
        r1: PackedCells, r2: PackedCells,
        r3: PackedCells, r4: PackedCells,
        r5: PackedCells, r6: PackedCells
    ) -> PackedCells {

        if !staggerred_right {            
            let mut new_cells = 0_u64;

            for offset in 0..=61 {
                let region = ((r1.0 & (0b111 << offset)) >> offset) // top 1x3 row
                    | ((r2.0 & (0b111 << offset)) >> offset << 3) // middle 1x3 row
                    | ((r3.0 & (0b111 << offset)) >> offset << 6); // bottom 1x3 row
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
            let mut new_cells = 0_u64;

            let leftmost = (r1.0 >> 62) | ((r4.0 & 0b1) << 2) 
                | ((r2.0 >> 62) << 3) | ((r5.0 & 0b1) << 5)
                | ((r3.0 >> 62) << 6) | ((r6.0 & 0b1) << 8);
            new_cells |= self.lookup(leftmost) as u64;

            let second_leftmost = (r1.0 >> 63) | ((r4.0 & 0b11) << 1)
                | ((r2.0 >> 63) << 3) | ((r5.0 & 0b11) << 4)
                | ((r3.0 >> 63) << 6) | ((r6.0 & 0b11) << 7);
            new_cells |= (self.lookup(second_leftmost) as u64) << 1;
            println!("r1: {:?} r2: {:?} r3: {:?} r4: {:?} r5: {:?} r6: {:?}", r1, r2, r3, r4, r5, r6);
            println!("leftmost: {:b}", leftmost);
            println!("second leftmost: {:b}", second_leftmost);
            for offset in 0..=61 {
                let region = (r4.0 & (0b111 << offset) >> offset) // top 1x3 row
                | (r5.0 & (0b111 << offset) >> offset << 3) // middle 1x3 row
                | (r6.0 & (0b111 << offset) >> offset << 6); // bottom 1x3 row
                println!("offset: {} {:b}", offset, region);
                new_cells |= (self.lookup(region) as u64) << offset << 2;
            }
            PackedCells(new_cells)
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
    staggered_cells: PackedCellMap,
    staggerred_right: bool,
    step_table: StepTable,
}

impl Universe {
    pub fn new() -> Universe {
        Universe {
            cells: PackedCellMap(FnvHashMap::default()),
            staggered_cells: PackedCellMap(FnvHashMap::default()),
            staggerred_right: false,
            step_table: StepTable::new(vec!(3), vec!(2)),
        }
    }

    pub fn step(&mut self) {
        let mut next_gen = PackedCellMap(FnvHashMap::default());
        for (&coordinates, &cells) in self.get_packed_cells() {
            let new_cells = self.new_cells(coordinates, cells);
            next_gen.0.insert(coordinates, new_cells);
            Universe::activate_neighbors(&mut next_gen, coordinates);
            ConsoleService::new().log(&format!("got cells {:?} {:?} ", new_cells, cells));
        }

        ConsoleService::new().log(&format!("cells {:?}", self.cells.0));
        ConsoleService::new().log(&format!("s cells{:?}", self.staggered_cells.0));

        ConsoleService::new().log(&format!("nextgen {:?}", next_gen.0));
        if self.staggerred_right {
            self.cells = next_gen;
        } else {
            self.staggered_cells = next_gen;
        }
        self.staggerred_right = !self.staggerred_right;
        ConsoleService::new().log(&format!("s{:?}", self.staggerred_right));

    }

    pub fn new_cells(&self, coordinates: PackedCoordinates, cells: PackedCells) -> PackedCells {
        if self.staggerred_right {
            let r1 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(1, 1))).cloned().unwrap_or(PackedCells::default());
            let r2 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(1, 0))).cloned().unwrap_or(PackedCells::default());
            let r3 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(1, 0) + PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r4 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r5 = cells;
            let r6 = self.staggered_cells.0.get(&(coordinates + PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            ConsoleService::new().log(&format!("staggered r1 {:?} r2 {:?} r3 {:?} r4 {:?} r5 {:?} r6 {:?}", r1, r2, r3, r4, r5, r6));
            self.step_table.step(self.staggerred_right, r1, r2, r3, r4, r5, r6)
        } else {
            let r1 = self.cells.0.get(&(coordinates - PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r2 = cells;
            let r3 = self.cells.0.get(&(coordinates + PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r4 = self.cells.0.get(&(coordinates - PackedCoordinates(0, 1) + PackedCoordinates(1, 0))).cloned().unwrap_or(PackedCells::default());
            let r5 = self.cells.0.get(&(coordinates + PackedCoordinates(1, 0))).cloned().unwrap_or(PackedCells::default());
            let r6 = self.cells.0.get(&(coordinates + PackedCoordinates(1, 1))).cloned().unwrap_or(PackedCells::default());
            ConsoleService::new().log(&format!("not staggered r1 {:?} r2 {:?} r3 {:?} r4 {:?} r5 {:?} r6 {:?}", r1, r2, r3, r4, r5, r6));

            self.step_table.step(self.staggerred_right, r1, r2, r3, r4, r5, r6)
        }
    }

    pub fn add(&mut self, coordinates: PackedCoordinates, cells: PackedCells) {
        if self.staggerred_right {
            self.staggered_cells.0.insert(coordinates, cells);
            Universe::activate_neighbors(&mut self.staggered_cells, coordinates);        
        } else {            
            self.cells.0.insert(coordinates, cells);
            Universe::activate_neighbors(&mut self.cells, coordinates);                    
        }
        
    }

    pub fn activate_neighbors(hash: &mut PackedCellMap, coordinates: PackedCoordinates) {

        let neighbor_coordinates = vec!(
            coordinates + PackedCoordinates(1, 0),
            coordinates + PackedCoordinates(1, 1),
            coordinates + PackedCoordinates(0, 1),
            coordinates - PackedCoordinates(1, 0),
            coordinates - PackedCoordinates(1, 1),
            coordinates - PackedCoordinates(0, 1),
            coordinates + PackedCoordinates(1, 0) - PackedCoordinates(0, 1),
            coordinates + PackedCoordinates(0, 1) - PackedCoordinates(1, 0),
        );

        for neighbor in neighbor_coordinates.into_iter() {
            if !hash.0.contains_key(&neighbor) {
                hash.0.insert(neighbor, PackedCells::default());
            }
        }
    }
    
    pub fn get_packed_cells(&self) -> Iter<PackedCoordinates, PackedCells> {
        if self.staggerred_right{
            self.staggered_cells.0.iter()
        } else {
            self.cells.0.iter()
        }
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
        let handle = interval.spawn(Duration::from_millis(5000), link.callback(|_| Msg::Tick));

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

        canvas.set_width(1000);
        canvas.set_height(1000);

        /*let start = PackedCoordinates(1_u32, 10_u32);
        let cells = PackedCells(0b1);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 11_u32);
        let cells = PackedCells(0b1);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 12_u32);
        let cells = PackedCells(0b1);
        self.universe.add(start, cells);*/
        let start = PackedCoordinates(1_u32, 20_u32);
        let cells = PackedCells(0b011);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 21_u32);
        let cells = PackedCells(0b11);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 20_u32);
        let cells = PackedCells(0b11);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 21_u32);
        let cells = PackedCells(0b11);
        self.universe.add(start, cells);
        let start = PackedCoordinates(0_u32, 20_u32);
        let cells = PackedCells(0b111 << 63);
        self.universe.add(start, cells);
        let start = PackedCoordinates(0_u32, 21_u32);
        let cells = PackedCells(0b111);
        self.universe.add(start, cells);
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
                self.universe.step();
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

        let cell_size = 3;

        ctx.begin_path();
        if self.universe.staggerred_right {
            ctx.set_fill_style(&JsValue::from_str("blue"));
        }
        ctx.set_fill_style(&JsValue::from_str("white"));

        ctx.fill_rect(0_f64, 0_f64, 2000_f64, 2000_f64);

        ctx.begin_path();
        ctx.set_fill_style(&JsValue::from_str("red"));


        for (coordinates, packed_cells) in self.universe.get_packed_cells() {
            let mut num = 0;
            let mut cells = packed_cells.0 as u64;
            while cells > 0 {
                if cells & 1_u64 == 1_u64 {
                    let mut x_start = ((coordinates.get_x() * 64 + num) * cell_size) as f64;
                    let mut y_start = (coordinates.get_y() * cell_size) as f64;
                    if !self.universe.staggerred_right {
                        x_start -= cell_size as f64;                        
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
    fn test_lookup() {
        let lookup = StepTable::new(vec![3], vec![2]);

        assert_eq!(0b1, lookup.lookup(0b010_010_010));
        assert_eq!(0b1, lookup.lookup(0b111_000_000));
        assert_eq!(0b0, lookup.lookup(0b111_111_111));
        assert_eq!(0b0, lookup.lookup(0b111_000_001));
    }

    #[test]
    fn test_step() {
        let r1 = PackedCells(0b1);
        let r4 = PackedCells(0); 
        let lookup = StepTable::new(vec![3], vec![2]);
        let expected = PackedCells(0b1);

        assert_eq!(expected, lookup.step(true, r1, r1, r1, r4, r4, r4));

        let expected = PackedCells(0xc000000000000000);
        assert_eq!(expected, lookup.step(true, r4, r4, r4, r1, r1, r1));

        let r1 = PackedCells(0x8000000000000000);
        let expected = PackedCells(0b11);
        assert_eq!(expected, lookup.step(false, r1, r1, r1, r4, r4, r4));

        let expected = PackedCells(0);
        assert_eq!(expected, lookup.step(false, r4, r4, r4, r1, r1, r1));
    }
    /*

    #[test]
    fn test_new_cells() {
        let mut universe = Universe::new();
        universe.add(PackedCoordinates(1, 1), PackedCells(0b111));
        universe.step();
        let expected = Some(&PackedCells(0b1));
        println!("{:?}", universe.cells.0);
        println!("{:?}", universe.staggered_cells.0);
        assert_eq!(expected, universe.staggered_cells.0.get(&PackedCoordinates(1, 1)));
        assert_eq!(expected,universe.staggered_cells.0.get(&PackedCoordinates(1, 2)));
        assert_eq!(expected,universe.staggered_cells.0.get(&PackedCoordinates(1, 0)));
    }*/
}