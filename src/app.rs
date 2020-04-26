use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, WheelEvent, WebGlBuffer, WebGlShader, WebGlProgram,WebGlUniformLocation};
use web_sys::WebGl2RenderingContext as GL;
use yew::services::{IntervalService, RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};

use std::time::Duration;

extern crate js_sys;

pub enum Msg {
    RenderGl,
    Step,
    PlayOrPause,
    StepIfNotPaused,
    Zoom(WheelEvent),
}
pub struct App {
    canvas: Option<HtmlCanvasElement>,
    gl: Option<GL>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    link: ComponentLink<Self>,
    #[allow(dead_code)]
    timer: Box<dyn Task>,
    universe: life::Universe,
    program: Option<WebGlProgram>,
    position_attribute_location: Option<u32>,
    position_buffer: Option<WebGlBuffer>,
    resolution_uniform_location: Option<WebGlUniformLocation>,
    color_uniform_location: Option<WebGlUniformLocation>,
    x: i32,
    y: i32,
    paused: bool,
    cell_size: f32,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut interval = IntervalService::new();
        let handle = interval.spawn(Duration::from_millis(1000 / 30), link.callback(|_| Msg::StepIfNotPaused));

        App {
            canvas: None,
            gl: None,
            link,
            node_ref: NodeRef::default(),
            render_loop: None,
            timer: Box::new(handle),
            universe: life::Universe::new(vec!(3), vec!(2, 3)),
            program: None,
            position_attribute_location: None,
            position_buffer: None,
            resolution_uniform_location: None,
            color_uniform_location: None,
            x: 0,
            y: 0,
            paused: true,
            cell_size: 5.0,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::RenderGl => {
                // Render functions are likely to get quite large, so it is good practice to split
                // it into it's own function rather than keeping it inline in the update match
                // case. This also allows for updating other UI elements that may be rendered in
                // the DOM like a framerate counter, or other overlaid textual elements.
                self.render_gl();
                false
            },
            Msg::Step => {
                self.universe.step();
                //self.universe.step();
                false
            },
            Msg::PlayOrPause => {
                self.paused = !self.paused;
                true
            },
            Msg::StepIfNotPaused => {
                if !self.paused {
                    self.universe.step();
                }
                false
            },
            Msg::Zoom(event) => {
                self.cell_size += event.delta_y() as f32 * -0.05;
                false
            }
        }        
    }
    fn mounted(&mut self) -> ShouldRender {
        // Once mounted, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas: HtmlCanvasElement = self.node_ref.cast::<HtmlCanvasElement>().unwrap();
        let mut gl: GL = canvas
            .get_context("webgl2")
            .expect("1")
            .expect("2")
            .dyn_into()
            .expect("3");

        self.initialize_gl(&mut gl);
        self.canvas = Some(canvas);
        self.gl = Some(gl);

        let render_frame = self.link.callback(|_| Msg::RenderGl);
        let handle = RenderService::new().request_animation_frame(render_frame);

        for x in 100..200 {
            for y in 100..200 {
                if js_sys::Math::random() < 0.2 {
                    self.universe.set_cell(x, y);
                }
            }
        }

        // A reference to the handle must be stored, otherwise it is dropped and the render won't
        // occur.
        self.render_loop = Some(Box::new(handle));
        // Since WebGL is rendered to the canvas "separate" from the DOM, there is no need to
        // render the DOM element(s) again.
        false
    }

    fn view(&self) -> Html {
        let play_or_pause = if self.paused {
            "Play"
        } else {
            "Pause"
        };

        html! {
                <div> 
                <button class="game-button" onclick=self.link.callback(|_| Msg::PlayOrPause)>{ play_or_pause }</button>
                <button class="game-button" onclick=self.link.callback(|_| Msg::Step)>{ "Step" }</button>
                <canvas ref={self.node_ref.clone()} onmousewheel=self.link.callback(|event| Msg::Zoom(event))>
                    { "This text is displayed if your browser does not support HTML5 Canvas." }
                </canvas>
                </div>
        }
    }
}

impl App {

    fn initialize_gl(&mut self, gl: &mut GL) {
        let vertex_code = include_str!("./life.vert");
        let fragment_code = include_str!("./life.frag");

        let vertex_shader = self.create_shader(gl, GL::VERTEX_SHADER, vertex_code);
        let fragment_shader = self.create_shader(gl, GL::FRAGMENT_SHADER, fragment_code);

        let program = self.create_program(gl, vertex_shader, fragment_shader);    

        // look up where the vertex data needs to go.
        let position_attribute_location = gl.get_attrib_location(&program, "a_position") as u32;
        self.position_attribute_location = Some(position_attribute_location);

        // look up uniform locations
        let resolution_uniform_location = gl.get_uniform_location(&program, "u_resolution");
        self.resolution_uniform_location = resolution_uniform_location;

        let color_uniform_location = gl.get_uniform_location(&program, "u_color");
        self.color_uniform_location = color_uniform_location;

        // Create a buffer to put three 2d clip space points in
        let position_buffer = gl.create_buffer();
        self.position_buffer = position_buffer;

        // Bind it to ARRAY_BUFFER (think of it as ARRAY_BUFFER = positionBuffer)
        gl.bind_buffer(GL::ARRAY_BUFFER, self.position_buffer.as_ref());
        self.program = Some(program);

        // turn off antialias 
        gl.get_context_attributes().unwrap().antialias(false);
    }

    fn create_shader(&self, gl: &mut GL, shader_type: u32, shader_source: &str) -> WebGlShader {
        let shader = gl.create_shader(shader_type).unwrap();
        gl.shader_source(&shader, shader_source);
        gl.compile_shader(&shader);
        let success = gl.get_shader_parameter(&shader, GL::COMPILE_STATUS);
        if success == JsValue::TRUE {
          return shader;
        }
       
        panic!("could not compile shader");
    }

    fn create_program(&self, gl: &mut GL, vertex_shader: WebGlShader, fragment_shader: WebGlShader) -> WebGlProgram {
        let program = gl.create_program().unwrap();
        gl.attach_shader(&program, &vertex_shader);
        gl.attach_shader(&program, &fragment_shader);
        gl.link_program(&program);
        let success = gl.get_program_parameter(&program, GL::LINK_STATUS);
        if success == JsValue::TRUE {
          return program;
        }
       
        panic!("could not create program");
    }
        
    pub fn render_gl(&mut self) {
        let gl = self.gl.as_ref().unwrap();
        let canvas = self.canvas.as_ref().unwrap();
        self.resize_gl();
        
        gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
        
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT);

        gl.use_program(self.program.as_ref());

          // Turn on the attribute
        gl.enable_vertex_attrib_array(self.position_attribute_location.unwrap());

        // Bind the position buffer.
        gl.bind_buffer(GL::ARRAY_BUFFER, self.position_buffer.as_ref());

        let size = 2;          // 2 components per iteration
        let gl_type = GL::FLOAT;   // the data is 32bit floats
        let normalize = false; // don't normalize the data
        let stride = 0;        // 0 = move forward size * sizeof(type) each iteration to get the next position
        let offset = 0;        // start at the beginning of the buffer
        gl.vertex_attrib_pointer_with_i32(
            self.position_attribute_location.unwrap(), size, gl_type, normalize, stride, offset);

        // set the resolution
        gl.uniform2f(self.resolution_uniform_location.as_ref(), canvas.width() as f32, canvas.height() as f32);
        // set the color
        gl.uniform4f (self.color_uniform_location.as_ref(), 0.0, 1.0, 0.0, 1.0);

        let vertices: Vec<f32> = self.collect_cells(self.x, self.y, self.x + canvas.width() as i32, self.y + canvas.height() as i32);
        let verts = js_sys::Float32Array::from(vertices.as_slice());
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &verts, GL::STATIC_DRAW);

        // Draw the rectangle
        let primitive_type = GL::TRIANGLES;
        let offset = 0;
        let count = vertices.len() as i32 / 2;

        gl.draw_arrays(primitive_type, offset, count as i32);
        let render_frame = self.link.callback(|_| Msg::RenderGl);
        let handle = RenderService::new().request_animation_frame(render_frame);
        // A reference to the new handle must be retained for the next render to run.
        self.render_loop = Some(Box::new(handle));
    }


    fn resize_gl(&self) {
        let canvas = self.canvas.as_ref().unwrap();
        // Lookup the size the browser is displaying the canvas.
        let display_width = canvas.client_width() as u32;
        let display_height = canvas.client_height() as u32;
        
        // Check if the canvas is not the same size.
        if canvas.width() != display_width || canvas.height() != display_height {
            // Make the canvas the same size
            canvas.set_width(display_width);
            canvas.set_height(display_height);
        }
    }

    fn collect_cells(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Vec<f32> {
        
        let mut vertices = Vec::new();
        let cell_size = self.cell_size;
        for cell in self.universe.live_cells() {
            let cell_x1 = cell.0 as f32 * cell_size;
            let cell_y1 = cell.1 as f32 * cell_size;
            let cell_x2 = cell_x1 + cell_size;
            let cell_y2 = cell_y1 + cell_size;
            if (cell_x1 > x1 as f32 || cell_x2 < x2 as f32) && (cell_y1 > y1 as f32 || cell_y2 < y2 as f32) {
                let cell_x1 = cell_x1 as f32 - x1 as f32;
                let cell_x2 = cell_x2 as f32 - x1 as f32;
                let cell_y1 = cell_y1 as f32 - y1 as f32;
                let cell_y2 = cell_y2 as f32 - y1 as f32;
                vertices.append(&mut vec!(
                            cell_x1, cell_y1,
                            cell_x2, cell_y1,
                            cell_x1, cell_y2,
                            cell_x1, cell_y2,
                            cell_x2, cell_y1,
                            cell_x2, cell_y2));
                }
            }
        vertices
    }
}


mod life {
    use fnv::{FnvHashMap, FnvHashSet};
    use std::ops::{Add, Sub};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum CellState {
        Alive,
        Dead,
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Tile(pub u32);

    // x grows to the right
    // y grows down
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct TCoord(pub i64, pub i64);

    pub type TMap = FnvHashMap<TCoord, Tile>;
    type TSet = FnvHashSet<TCoord>;

    struct RuleTable(Box<[u32]>);

    pub struct Universe {
        pub p01: TMap,
        pub p10: TMap,
        active: TSet,
        pub generation: u64,
        rule_table: RuleTable,
    }

    impl Add for TCoord {
        type Output = Self;

        fn add(self, other: Self) -> Self {
            TCoord(self.0.wrapping_add(other.0), self.1.wrapping_add(other.1))
        }
    }

    impl Sub for TCoord {
        type Output = Self;

        fn sub(self, other: Self) -> Self {
            TCoord(self.0.wrapping_sub(other.0), self.1.wrapping_sub(other.1))
        }
    }

    impl RuleTable {

        fn new(b: Vec<u32>, s: Vec<u32>) -> RuleTable {
            let mut table = vec![0; 65536];

            for idx in 0..table.len() {
                let i = idx as u32;
                let lr_s = i & 0x777;
                let lr_b = lr_s & !0x20;
                let ll_s = i & 0xeee;
                let ll_b = ll_s & !0x40;
                let ur_s = i & 0x7770;
                let ur_b = ur_s & !0x200;
                let ul_s = i & 0xeee0;
                let ul_b = ul_s & !0x400;
                
                let lr = if (((lr_s & 0x20) != 0) && s.contains(&lr_b.count_ones())) || b.contains(&lr_b.count_ones()) {
                    1
                } else {
                    0
                };

                let ll = if (((ll_s & 0x40) != 0) && s.contains(&ll_b.count_ones())) || b.contains(&ll_b.count_ones()) {
                    1
                } else {
                    0
                };

                let ur = if (((ur_s & 0x200) != 0) && s.contains(&ur_b.count_ones())) || b.contains(&ur_b.count_ones()) {
                    1
                } else {
                    0
                };

                let ul = if (((ul_s & 0x400) != 0) && s.contains(&ul_b.count_ones())) || b.contains(&ul_b.count_ones()) {
                    1
                } else {
                    0
                };

                let result: u32 = lr + (ll << 1) + (ur << 4) + (ul << 5);                
                table[idx] = result;
            }
            RuleTable(table.into_boxed_slice())
        }

    }

    impl Default for RuleTable {

        fn default() -> RuleTable {
            RuleTable::new(vec!(3), vec!(2, 3))
        }
    }

    impl std::fmt::Debug for Tile {

        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{:b}", self.0)
        }
    }

    impl Universe {

        pub fn new(b: Vec<u32>, s: Vec<u32>) -> Universe {
            let p01 = TMap::default();
            let p10 = TMap::default();
            let active = TSet::default();
            let generation = 0;
            let rule_table = RuleTable::new(b, s);

            Universe {
                p01, p10, active, generation, rule_table,
            }
        }

        pub fn step(&mut self) {
            let is_even = self.generation % 2 == 0;
            if is_even {
                let (active, p10) = self.p01_step(&self.active, &self.p01);
                self.active = active;
                self.p10 = p10;
            } else {
                let (active, p01) = self.p10_step(&self.active, &self.p10);
                self.active = active;
                self.p01 = p01;
            }

            self.generation += 1;
        }

        fn p01_step(&self, active: &TSet, tiles: &TMap) -> (TSet, TMap) {
            let mut next_active = TSet::default();
            let mut next_gen = TMap::default();

            for coord in active {
                let right_coord = *coord + TCoord(1, 0);
                let down_coord = *coord + TCoord(0, 1);
                let downright_coord = *coord + TCoord(1, 1);

                let tile = tiles.get(coord).cloned().unwrap_or(Tile(0));
                let right = tiles.get(&right_coord).cloned().unwrap_or(Tile(0));
                let down = tiles.get(&down_coord).cloned().unwrap_or(Tile(0));
                let downright = tiles.get(&downright_coord).cloned().unwrap_or(Tile(0));

                let new_tile = self.p01_calc(tile, right, down, downright);
                next_gen.insert(*coord, new_tile);

                if tile != new_tile {
                    next_active.insert(*coord);
                    next_active.insert(right_coord);
                    next_active.insert(down_coord);
                    next_active.insert(downright_coord);
                }
                
            }
            (next_active, next_gen)
        }

        fn p10_step(&self, active: &TSet, tiles: &TMap) -> (TSet, TMap) {
            let mut next_active = TSet::default();
            let mut next_gen = TMap::default();

            for coord in active {
                let left_coord = *coord - TCoord(1, 0);
                let up_coord = *coord - TCoord(0, 1);
                let upleft_coord = *coord - TCoord(1, 1);

                let tile = tiles.get(coord).cloned().unwrap_or(Tile(0));
                let left = tiles.get(&left_coord).cloned().unwrap_or(Tile(0));
                let up = tiles.get(&up_coord).cloned().unwrap_or(Tile(0));
                let upleft = tiles.get(&upleft_coord).cloned().unwrap_or(Tile(0));

                let new_tile = self.p10_calc(tile, left, up, upleft);
                next_gen.insert(*coord, new_tile);
                if tile != new_tile {
                    next_active.insert(*coord);
                    next_active.insert(left_coord);
                    next_active.insert(up_coord);
                    next_active.insert(upleft_coord);
                }
                
            }
            (next_active, next_gen)
        }

        fn p01_calc(&self, tile: Tile, right: Tile, down: Tile, downright: Tile) -> Tile {

            let center_data = tile.0;
            let down_data = (center_data << 8) + (down.0 >> 24);
            let right_data = ((center_data << 2) & 0xcccccccc) + ((right.0 >> 2) & 0x33333333);
            let downright_data = ((down_data << 2) & 0xcccccccc) + ((((right.0 << 8) + (downright.0 >> 24)) >> 2) & 0x33333333);

            Tile(
                (self.rule_table.0[(center_data >> 16) as usize] << 26) +
                (self.rule_table.0[(down_data >> 16) as usize] << 18) +
                (self.rule_table.0[(center_data & 0xffff) as usize] << 10) +
                (self.rule_table.0[(down_data & 0xffff) as usize] << 2) +
                (self.rule_table.0[(right_data >> 16) as usize] << 24) +
                (self.rule_table.0[(downright_data >> 16) as usize] << 16) +
                (self.rule_table.0[(right_data & 0xffff) as usize] << 8) +
                (self.rule_table.0[(downright_data & 0xffff) as usize])
            )
        }

        fn p10_calc(&self, tile: Tile, left: Tile, up: Tile, upleft: Tile) -> Tile {

            let center_data = tile.0;
            let up_data = (center_data >> 8) + (up.0 << 24);
            let left_data = ((center_data >> 2) & 0x33333333) + ((left.0 << 2) & 0xcccccccc) ;
            let leftup_data = ((up_data >> 2) & 0x33333333) + ((((left.0 >> 8) + (upleft.0 << 24)) << 2) & 0xcccccccc);

            Tile(
                (self.rule_table.0[(leftup_data >> 16) as usize] << 26) +
                (self.rule_table.0[(left_data >> 16) as usize] << 18) +
                (self.rule_table.0[(leftup_data & 0xffff) as usize] << 10) +
                (self.rule_table.0[(left_data & 0xffff) as usize] << 2) +
                (self.rule_table.0[(up_data >> 16) as usize] << 24) +
                (self.rule_table.0[(center_data >> 16) as usize] << 16) +
                (self.rule_table.0[(up_data & 0xffff) as usize] << 8) +
                (self.rule_table.0[(center_data & 0xffff) as usize])
            )
        }

        fn get_cell(&self, mut x: i64, mut y: i64) -> CellState {
            // test both odd and even
            // test (0, 0), (-1, 0), (-1,-1), (-1, 2)
            // test 2^32, -2^32-1

            if self.generation % 2 == 1 {
                x -= 1;
                y -= 1;
            }

            let coord_x = if x > 0 {
                x / 4
            } else {
                (x / 4) + ((x % 4) - 3) / 4
            };

            let coord_y = if y > 0 {
                y / 8
            } else {
                (y / 8) + ((y % 8) - 7) / 8
            };

            let coord = TCoord(coord_x, coord_y);
            
            let tile = if self.generation % 2 == 0 {
                self.p01.get(&coord).cloned().unwrap_or(Tile(0))
            } else {
                self.p10.get(&coord).cloned().unwrap_or(Tile(0))
            };


            let t_x = if x >= 0 {
                x % 4
            } else {
                ((x % 4) + 4) % 4
            };

            let t_y = if y >= 0 {
                y % 8
            } else {
                ((y % 8) + 8) % 8
            };

            if tile.0 >> (31 - 4*t_y - t_x) & 1 != 0 {
                CellState::Alive
            } else {
                CellState::Dead
            }
        }

        pub fn set_cell(&mut self, mut x: i64, mut y: i64) {
            // test both odd and even
            // test (0, 0), (-1, 0), (-1,-1), (-1, 2)
            // test 2^32, -2^32-1

            if self.generation % 2 == 1 {
                x -= 1;
                y -= 1;
            }

            let coord_x = if x >= 0 {
                x / 4
            } else {
                (x / 4) + ((x % 4) - 3) / 4
            };

            let coord_y = if y >= 0 {
                y / 8
            } else {
                (y / 8) + ((y % 8) - 7) / 8
            };

            let coord = TCoord(coord_x, coord_y);
            
            let mut tile = if self.generation % 2 == 0 {
                self.p01.get(&coord).cloned().unwrap_or(Tile(0))
            } else {
                self.p10.get(&coord).cloned().unwrap_or(Tile(0))
            };


            let t_x = if x >= 0 {
                x % 4
            } else {
                ((x % 4) + 4) % 4
            };

            let t_y = if y >= 0 {
                y % 8
            } else {
                ((y % 8) + 8) % 8
            };
            
            tile.0 |= 1 << (31 - 4*t_y - t_x);

            if self.generation % 2 == 0 {
                self.p01.insert(coord, tile);
                self.active.insert(coord);
                self.active.insert(coord - TCoord(1, 1));
                self.active.insert(coord - TCoord(1, 0));
                self.active.insert(coord - TCoord(0, 1));
            } else {
                self.p10.insert(coord, tile);
                self.active.insert(coord);
                self.active.insert(coord + TCoord(1, 1));
                self.active.insert(coord + TCoord(1, 0));
                self.active.insert(coord + TCoord(0, 1));
            }
        }

        pub fn live_cells(&self) -> Vec<(i64, i64)> {
            let mut cells = Vec::new();

            if self.generation % 2 == 1 {
                for (coord, cell) in &self.p10 {
                    let mut cell = cell.0;
                    let mut num_shifts = 0;
                    while cell != 0 {
                        if cell & 1 == 1 {
                            let x = coord.0 * 4 + (3 - (num_shifts % 4));
                            let y = coord.1 * 8 + (7 - (num_shifts / 4));
                            cells.push((x + 1, y + 1));
                        }
                        num_shifts += 1;
                        cell >>= 1;
                    }
                }
            } else {
                for (coord, cell) in &self.p01 {
                    let mut cell = cell.0;
                    let mut num_shifts = 0;
                    while cell != 0 {
                        if cell & 1 == 1 {
                            let x = coord.0 * 4 + (3 - (num_shifts % 4));
                            let y = coord.1 * 8 + (7 - (num_shifts / 4));
                            cells.push((x, y));
                        }
                        num_shifts += 1;
                        cell >>= 1;
                    }
                }
            }
            cells
        }

        // use include_str! for popular patterns
    }

    impl Default for Universe {

        fn default() -> Universe {
            let b = vec!(3);
            let s = vec!(2, 3);

            Universe::new(b, s)
        }
    }


    #[cfg(test)]
    mod test {

        use super::*;

        #[test]
        fn test_p01_calc() {
            let universe = Universe::default();
            let tile = Tile(0xc800_2220);
            let right = Tile(0x8880_0008);
            let down = Tile(0xd200_0000);
            let downright = Tile(0x8000_0000);

            let expected = Tile(0xb000_e2df);
            let result = universe.p01_calc(tile, right, down, downright);

            assert_eq!(expected, result);
        }

        #[test]
        fn test_p10_calc() {
            let universe = Universe::default();
            let tile = Tile(0x0044_4013);
            let left = Tile(0x3100_0013);
            let up = Tile(0x18);
            let upleft = Tile(0x81);

            let expected = Tile(0xc0c47009);
            let result = universe.p10_calc(tile, left, up, upleft);

            assert_eq!(expected, result);
        }

        #[test]
        fn test_step() {
            let mut universe = Universe::default();
            universe.set_cell(1, 1);
            universe.set_cell(2, 1);
            universe.set_cell(3, 1);
            universe.step();

            assert_eq!(CellState::Dead, universe.get_cell(1, 1));
            assert_eq!(CellState::Alive, universe.get_cell(2, 1));
            assert_eq!(CellState::Dead, universe.get_cell(3, 1));
            assert_eq!(CellState::Alive, universe.get_cell(2, 2));
            assert_eq!(CellState::Alive, universe.get_cell(2, 0));

            universe.step();
            assert_eq!(CellState::Alive, universe.get_cell(1, 1));
            assert_eq!(CellState::Alive, universe.get_cell(2, 1));
            assert_eq!(CellState::Alive, universe.get_cell(3, 1));

        }

        #[test]
        fn test_many_steps() {
            let mut universe = Universe::default();
            universe.set_cell(0, 0);
            universe.set_cell(1, 0);
            universe.set_cell(2, 0);
            universe.set_cell(2, -1);
            universe.set_cell(1, -2);
            
            for _i in 0..1000 {
                universe.step();
            }

            let expected = 5;
            assert_eq!(expected, universe.p01.values().map(|v| v.0.count_ones()).sum::<u32>());
        }

        extern crate test;    
        use test::{Bencher, black_box};

        #[bench]
        fn bench_p01_calc(b: &mut Bencher) {
            let universe = Universe::default();
            let tile = Tile(0xc800_2220);
            let right = Tile(0x8880_0008);
            let down = Tile(0xd200_0000);
            let downright = Tile(0x8000_0000);

            b.iter(|| {
                // Inner closure, the actual test
                for _i in 1..100 {
                    black_box(universe.p01_calc(tile, right, down, downright));
                }
            });
        }
    }
}   