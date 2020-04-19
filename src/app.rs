use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::services::{ConsoleService, IntervalService, RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};

use fnv::FnvHashMap;
use std::time::Duration;
use std::collections::hash_map::Iter;

extern crate js_sys;


const RANDOM_ALIVE: f64 = 0.2;

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

impl PackedCells {

    pub fn stagger_right(
        &self,
        transitions: &Transitions,
        top: PackedCells,
        bottom: PackedCells,
        upper_right: PackedCells,
        right: PackedCells,
        bottom_right: PackedCells,
    ) -> PackedCells {

        let mut result = 0_u64;

        for offset in 0..=61 {
            let region = ((top.0 & (0b111 << offset)) >> offset) // top 1x3 row
                | ((self.0 & (0b111 << offset)) >> offset << 3) // middle 1x3 row
                | ((bottom.0 & (0b111 << offset)) >> offset << 6); // bottom 1x3 row
            result |= (transitions.tf(region) as u64) << offset;
        }
        

        let second_rightmost = (top.0 >> 62) | ((upper_right.0 & 0b1) << 2)
            | ((self.0 >> 62) << 3) | ((right.0 & 0b1) << 5)
            | ((bottom.0 >> 62) << 6) | ((bottom_right.0 & 0b1) << 8);
        
        result |= (transitions.tf(second_rightmost) as u64) << 62;

        let rightmost = (top.0 >> 63) | ((upper_right.0 & 0b11) << 1) // top 1x3 row
        | ((self.0 >> 63) << 3) | ((right.0 & 0b11) << 4) // middle 1x3 row
        | ((bottom.0 >> 63) << 6) | ((bottom_right.0 & 0b11) << 7); // bottom 1x3 row
        result |= (transitions.tf(rightmost) as u64) << 63;

        PackedCells(result)
    }

    pub fn stagger_left(
        &self,
        transitions: &Transitions,
        upper_left: PackedCells,
        left: PackedCells,
        bottom_left: PackedCells,
        top: PackedCells,
        bottom: PackedCells,
    ) -> PackedCells {

        let mut new_cells = 0_u64;

        let leftmost = ((upper_left.0 & (0b01 << 62)) >> 62) | ((upper_left.0 >> 63) << 1) | ((top.0 & 0b1) << 2)
            | ((left.0 & (0b01 << 62)) >> 59) | ((left.0 >> 63) << 4) | ((self.0 & 0b1) << 5)
            | ((bottom_left.0 & (0b01 << 62)) >> 56) | ((bottom_left.0 >> 63) << 7) | ((bottom.0 & 0b1) << 8);
        new_cells |= transitions.tf(leftmost) as u64;

        let second_leftmost = ((upper_left.0 & (0b1 << 63)) >> 63) | ((top.0 & 0b11) << 1)
            | ((left.0 & (0b1 << 63)) >> 60) | ((self.0 & 0b11) << 4)
            | (((bottom_left.0 & (0b1 << 63)) >> 57)) | ((bottom.0 & 0b11) << 7);

        new_cells |= (transitions.tf(second_leftmost) as u64) << 1;

        for offset in 0..=61 {
            let region = ((top.0 & (0b111 << offset)) >> offset) // top 1x3 row
            | (((self.0 & (0b111 << offset)) >> offset) << 3) // middle 1x3 row
            | (((bottom.0 & (0b111 << offset)) >> offset) << 6); // bottom 1x3 row

            new_cells |= (transitions.tf(region) as u64) << offset << 2;
        }        
        /*if self.0 == 0b10 {
            println!("stagger left: {:b} {:b} left: {:b} 2ndleft: {:b}", self.0, new_cells, leftmost, second_leftmost);
            println!("stagger left: ul {:b} l: {:b} bl: {:b} t: {:b} s: {:b} b: {:b}", upper_left.0, left.0, bottom_left.0, top.0, self.0, bottom.0)
        }*/
        PackedCells(new_cells)
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
pub struct Transitions(Box<[u8]>);

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

impl Transitions {

    pub fn new(b: Vec<u32>, s: Vec<u32>) -> Self {
        let tf = Self::make_transition_function(b, s);
        Self(tf)
    }

    pub fn conway() -> Self {
        let tf = Self::make_transition_function(vec!(3), vec!(2));
        Self(tf)
    }

    pub fn make_transition_function(b: Vec<u32>, s: Vec<u32>) -> Box<[u8]> {
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
        }
        lookup.into_boxed_slice()
    }

    pub fn tf(&self, region: u64) -> u8 {
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
    transitions: Transitions,
}

impl Universe {
    pub fn new() -> Universe {
        Universe {
            cells: PackedCellMap(FnvHashMap::default()),
            staggered_cells: PackedCellMap(FnvHashMap::default()),
            staggerred_right: false,
            transitions: Transitions::conway(),
        }
    }

    pub fn randomize(&mut self, x_start: u32, y_start: u32, x_end: u32, y_end: u32) {
        for x in x_start..=x_end {

            //ConsoleService::new().log(&format!("got x {} ", x));
            for y in y_start..=y_end {

                let mut num = 0_u64;
                for shift in 0..=63 {
                    if js_sys::Math::random() < RANDOM_ALIVE {
                        num |= 0b1 << shift;
                    }
                }
                self.add(PackedCoordinates(x, y), PackedCells(num));
                //ConsoleService::new().log(&format!("got cells {:?}  {:?}", PackedCells(num), PackedCoordinates(x, y)));
            }
        }
    }

    pub fn step(&mut self) {
        let mut next_gen = PackedCellMap(FnvHashMap::default());
        for (&coordinates, &cells) in self.get_packed_cells() {
            let (new_cells, empty) = self.new_cells(coordinates, cells);
            if !empty {
                next_gen.0.insert(coordinates, new_cells);
                Universe::activate_neighbors(&mut next_gen, coordinates);

            }
            //if new_cells.0 != 0 {
            //}
            ////ConsoleService::new().log(&format!("got cells {:?} {:?} ", new_cells, cells));
        }

        ////ConsoleService::new().log(&format!("cells {:?}", self.cells.0));
        ////ConsoleService::new().log(&format!("s cells{:?}", self.staggered_cells.0));

        //ConsoleService::new().log(&format!("nextgen {:?}", next_gen.0));
        if self.staggerred_right {
            self.cells = next_gen;
        } else {
            self.staggered_cells = next_gen;
        }
        self.staggerred_right = !self.staggerred_right;
        ////ConsoleService::new().log(&format!("s{:?}", self.staggerred_right));

    }

    pub fn new_cells(&self, coordinates: PackedCoordinates, cells: PackedCells) -> (PackedCells, bool) {
        if self.staggerred_right {
            let r1 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(1, 1))).cloned().unwrap_or(PackedCells::default());
            let r2 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(1, 0))).cloned().unwrap_or(PackedCells::default());
            let r3 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(1, 0) + PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r4 = self.staggered_cells.0.get(&(coordinates - PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r5 = cells;
            let r6 = self.staggered_cells.0.get(&(coordinates + PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            if r1.0 == 0 && r2.0 == 0 && r3.0 == 0 && r4.0 == 0 && r5.0 == 0 && r6.0 == 0 {
                return (PackedCells::default(), true)
            }
            ////ConsoleService::new().log(&format!("staggered r1 {:?} r2 {:?} r3 {:?} r4 {:?} r5 {:?} r6 {:?}", r1, r2, r3, r4, r5, r6));
            (r5.stagger_left(&self.transitions, r1, r2, r3, r4, r6), false)
        } else {
            let r1 = self.cells.0.get(&(coordinates - PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r2 = cells;
            let r3 = self.cells.0.get(&(coordinates + PackedCoordinates(0, 1))).cloned().unwrap_or(PackedCells::default());
            let r4 = self.cells.0.get(&(coordinates - PackedCoordinates(0, 1) + PackedCoordinates(1, 0))).cloned().unwrap_or(PackedCells::default());
            let r5 = self.cells.0.get(&(coordinates + PackedCoordinates(1, 0))).cloned().unwrap_or(PackedCells::default());
            let r6 = self.cells.0.get(&(coordinates + PackedCoordinates(1, 1))).cloned().unwrap_or(PackedCells::default());
            ////ConsoleService::new().log(&format!("not staggered r1 {:?} r2 {:?} r3 {:?} r4 {:?} r5 {:?} r6 {:?}", r1, r2, r3, r4, r5, r6));
            if r1.0 == 0 && r2.0 == 0 && r3.0 == 0 && r4.0 == 0 && r5.0 == 0 && r6.0 == 0 {
                return (PackedCells::default(), true)
            }
            (r2.stagger_right(&self.transitions, r1, r3, r4, r5, r6), false)
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
        let handle = interval.spawn(Duration::from_millis(40), link.callback(|_| Msg::Tick));

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

        canvas.set_width(5000);
        canvas.set_height(5000);

        /*let start = PackedCoordinates(1_u32, 10_u32);
        let cells = PackedCells(0b1);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 11_u32);
        let cells = PackedCells(0b1);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 12_u32);
        let cells = PackedCells(0b1);
        self.universe.add(start, cells);*/
        /*let start = PackedCoordinates(1_u32, 20_u32);
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
        let start = PackedCoordinates(2_u32, 21_u32);
        let cells = PackedCells(0b010_000_1);
        self.universe.add(start, cells);
        let start = PackedCoordinates(2_u32, 22_u32);
        let cells = PackedCells(0b101_000_1);
        self.universe.add(start, cells);
        let start = PackedCoordinates(2_u32, 23_u32);
        let cells = PackedCells(0b10_000_1);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 21_u32);
        let cells = PackedCells(0b111_0_111);
        self.universe.add(start, cells);*/

        self.universe.randomize(0, 0, 20, 1000);
        let start = PackedCoordinates(1_u32, 5_u32);
        let cells = PackedCells(0b0100000000000000000000000000000000000000000000000000000000000000);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 6_u32);
        let cells = PackedCells(0b1000000000000000000000000000000000000000000000000000000000000000);
        self.universe.add(start, cells);
        let start = PackedCoordinates(1_u32, 7_u32);
        let cells = PackedCells(0b1110000000000000000000000000000000000000000000000000000000000000);
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
                for _i in 0..1 {
                    self.universe.step();
                }
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

        let cell_size = 5;

        ctx.begin_path();
        ctx.set_fill_style(&JsValue::from_str("white"));

        ctx.fill_rect(0_f64, 0_f64, 5000_f64, 5000_f64);

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

/*#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_lookup() {
        let transitions = Transitions::conway();

        assert_eq!(0b1, transitions.tf(0b010_010_010));
        assert_eq!(0b1, transitions.tf(0b111_000_000));
        assert_eq!(0b0, transitions.tf(0b111_111_111));
        assert_eq!(0b0, transitions.tf(0b111_000_001));
    }

    #[test]
    fn test_stagger_right() {
        let transitions = Transitions::conway();
        let cells = PackedCells(0b101010101010101);
        let default = PackedCells::default();

        let expected = PackedCells(0);
        let actual = cells.stagger_right(&transitions, default, default, default, default, default);
        assert_eq!(expected, actual);

        let cells = PackedCells(0b111);
        let expected = PackedCells(0b1);
        let actual = cells.stagger_right(&transitions, default, default, default, default, default);
        assert_eq!(expected, actual);

        let cells = PackedCells(0b111 << 61);
        
        let expected = PackedCells(0b001 << 61);
        let actual = cells.stagger_right(&transitions, default, default, default, default, default);
        assert_eq!(expected, actual);

        let cells = default;
        let upper_right = PackedCells(0b1);
        let right = PackedCells(0b1);
        let bottom_right = PackedCells(0b1);
        
        let expected = PackedCells(0b11 << 62);
        let actual = cells.stagger_right(&transitions, default, default, upper_right, right, bottom_right);
        assert_eq!(expected, actual);

        let cells = PackedCells(0b10);
        let top = PackedCells(0b10);
        let bottom = PackedCells(0b10);

        let expected = PackedCells(0b11);
        let actual = cells.stagger_right(&transitions, top, bottom, default, default, default);
        assert_eq!(expected, actual);        

        let cells = PackedCells(0b100);
        let top = PackedCells(0b100);
        let bottom = PackedCells(0b100);

        let expected = PackedCells(0b111);
        let actual = cells.stagger_right(&transitions, top, bottom, default, default, default);
        assert_eq!(expected, actual);      
    }

    #[test]
    fn test_stagger_left() {
        let transitions = Transitions::conway();
        let cells = PackedCells(0b101010101010101);
        let default = PackedCells::default();

        let expected = PackedCells(0);
        let actual = cells.stagger_left(&transitions, default, default, default, default, default);

        let cells = PackedCells(0b111);
        let expected = PackedCells(0b100);
        let actual = cells.stagger_left(&transitions, default, default, default, default, default);
        assert_eq!(expected, actual);

        let cells = PackedCells(0b111 << 61);
        
        let expected = PackedCells(0b1 << 63);
        let actual = cells.stagger_left(&transitions, default, default, default, default, default);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_step() {
        let mut universe = Universe::new();

        // what happens to a glider?
        universe.add(PackedCoordinates(0, 0), PackedCells(0b010));
        universe.add(PackedCoordinates(0, 1), PackedCells(0b001));
        universe.add(PackedCoordinates(0, 2), PackedCells(0b111));

        for i in 0..1000 {
            universe.step();
            println!("{:?}", i);
            println!("cells {:?}", universe.cells.0);
            println!("staggered {:?}", universe.staggered_cells.0);
            assert_eq!(universe.cells.0.values().map(|i| i.0.count_ones()).sum::<u32>(), 5);
            assert_eq!(universe.staggered_cells.0.values().map(|i| i.0.count_ones()).sum::<u32>(), 5);

        }
    }

    extern crate test;    
    use test::{Bencher, black_box};

    #[bench]
    fn bench_stagger_left(b: &mut Bencher) {
        let transitions = Transitions::conway();
        let default = PackedCells::default();
        let cells = PackedCells(0b111);

        b.iter(|| {
            // Inner closure, the actual test
            for i in 1..100 {
                cells.stagger_left(&transitions, default, default, default, default, default);
            }
        });
    }

    #[bench]
    fn bench_hashmap(b: &mut Bencher) {
        let coordinates = PackedCoordinates(30, 49);
        let mut universe = Universe::new();
        universe.add(PackedCoordinates(11, 12), PackedCells(0b11111));
        b.iter(|| {
            // Inner closure, the actual test
            for i in 1..100 {
                universe.staggered_cells.0.get(&(coordinates - PackedCoordinates(1, 1))).cloned().unwrap_or(PackedCells::default());
            }
        });
    }

    #[bench]
    fn bench_tf(b: &mut Bencher) {
        let transitions = Transitions::conway();

        let region = 0b110101_u64;
        b.iter(|| {
            for i in 1..6400 {
                transitions.tf(region);
            }
        });
    }

    #[bench]
    fn bench_3x3_bits(b: &mut Bencher) {
        let top = PackedCells(0b1111_0111_0000);
        let middle = PackedCells(0b0000_1111_0010);
        let bottom = PackedCells(0b0001_0111_1110);
        let transitions = Transitions::conway();

        b.iter(|| {
            for i in 1..100 {
                let mut new_cells = 0_u64;
                for offset in 0..=61 {
                    let region = ((top.0 & (0b111 << offset)) >> offset) // top 1x3 row
                    | (((middle.0 & (0b111 << offset)) >> offset) << 3) // middle 1x3 row
                    | (((bottom.0 & (0b111 << offset)) >> offset) << 6); // bottom 1x3 row
        
                    new_cells |= (transitions.tf(region) as u64) << offset << 2;
                } 
            }
        })
    }


    #[bench]
    fn bench_4x4_bits(b: &mut Bencher) {
        let top = PackedCells(0b1111_0111_0000);
        let middle = PackedCells(0b0000_1111_0010);
        let middle2 = PackedCells(0b11);
        let bottom = PackedCells(0b0001_0111_1110);
        let transitions = vec![0b1111; 65512];
        b.iter(|| {
            for i in 1..100 {
                let mut new_cells = 0_u64;
                for offset in (0..=58).step_by(2) {
                    let region = ((top.0 & (0b1111 << offset)) >> offset) // top 1x4 row
                    | (((middle.0 & (0b1111 << offset)) >> offset) << 4) // middle 1x4 row
                    | (((middle2.0 & (0b1111 << offset)) >> offset) << 8) // middle 1x4 row
                    | (((bottom.0 & (0b1111 << offset)) >> offset) << 12); // bottom 1x4 row
        
                    new_cells |= (transitions[region as usize] as u64) << offset << 2;
                } 
            }
        })
    }

    #[bench]
    fn bench_4x4_512_bits(b: &mut Bencher) {
        let top = PackedCells(0b1111_0111_0000);
        let middle = PackedCells(0b0000_1111_0010);
        let middle2 = PackedCells(0b11);
        let bottom = PackedCells(0b0001_0111_1110);
        let transitions = vec![0b1111; 65512];
        b.iter(|| {
            for i in 1..100 {
                let mut new_cells = 0_u64;
                for offset in (0..=58).step_by(2) {
                    let region = ((top.0 & (0b1111 << offset)) >> offset) // top 1x4 row
                    | (((middle.0 & (0b1111 << offset)) >> offset) << 4) // middle 1x4 row
                    | (((middle2.0 & (0b1111 << offset)) >> offset) << 8) // middle 1x4 row
                    | (((bottom.0 & (0b1111 << offset)) >> offset) << 12); // bottom 1x4 row
        
                    new_cells |= (transitions[region as usize] as u64) << offset << 2;
                } 
            }
        })
    }


    #[bench]
    fn bench_4x4_512_bits2(b: &mut Bencher) {
        let zisdata: u32 = 0b1111_0111_0000;
        let underdata: u32 = 0b0000_1111_0010;
        let otherdata: u32 = ((zisdata << 2) & 0xcccccccc) + ((zisdata >> 2) & 0x33333333) ;
        let otherunderdata: u32 = 0b0001_0111_1110;
        let ruletable: Vec<u32> = vec![0b1111; 65512];
        b.iter(|| {
            for _i in 1..100 {
                ruletable[(zisdata >> 16) as usize] << 26 +
                          (ruletable[(underdata >> 16) as usize] << 18) +
                          (ruletable[(zisdata & 0xffff) as usize] << 10) +
                          (ruletable[(underdata & 0xffff) as usize] << 2) +
                          (ruletable[(otherdata >> 16) as usize] << 24) +
                          (ruletable[(otherunderdata >> 16) as usize] << 16) +
                          (ruletable[(otherdata & 0xffff) as usize] << 8) +
                           ruletable[(otherunderdata & 0xffff) as usize] ;

            }
        })
    }

    #[bench]
    fn bench_4x4_512_64bits2(b: &mut Bencher) {
        let zisdata: u64 = 0b1111_0111_0000;
        let underdata: u64 = 0b0000_1111_0010;
        let otherdata: u64 = ((zisdata << 2) & 0xcccccccc) + ((zisdata >> 2) & 0x33333333) ;
        let otherunderdata: u64 = 0b0001_0111_1110;
        let ruletable: Vec<u64> = vec![0b1111; 65512];
        b.iter(|| {
            for _i in 1..100 {
                ruletable[(zisdata >> 16) as usize] << 26 +
                          (ruletable[(underdata >> 16) as usize] << 18) +
                          (ruletable[(zisdata & 0xffff) as usize] << 10) +
                          (ruletable[(underdata & 0xffff) as usize] << 2) +
                          (ruletable[(otherdata >> 16) as usize] << 24) +
                          (ruletable[(otherunderdata >> 16) as usize] << 16) +
                          (ruletable[(otherdata & 0xffff) as usize] << 8) +
                           ruletable[(otherunderdata & 0xffff) as usize] +
                           ruletable[(zisdata >> 48) as usize] << 48 +
                          (ruletable[(underdata >> 48) as usize] << 40) +
                          (ruletable[(zisdata & 0xffff) as usize] << 42) +
                          (ruletable[(underdata & 0xffff) as usize] << 34) +
                          (ruletable[(otherdata >> 48) as usize] << 56) +
                          (ruletable[(otherunderdata >> 48) as usize] << 48) +
                          (ruletable[(otherdata & 0xffff) as usize] << 40) +
                           ruletable[(otherunderdata & 0xffff) as usize] << 32
            
                           ;

            }
        })
    }
}   */