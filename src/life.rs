
/*mod life {
    use fnv::{FnvHashMap, FnvHashSet};
    use std::ops::{Add, Sub};
    use web_sys::WebGl2RenderingContext as GL;
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Element, WebGlBuffer, WebGlProgram, WebGlUniformLocation};
    use yew::NodeRef;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum CellState {
        Alive,
        Dead,
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    struct Tile(u32);

    // x grows to the right
    // y grows down
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    struct TCoord(i64, i64);

    type TMap = FnvHashMap<TCoord, Tile>;
    type TSet = FnvHashSet<TCoord>;

    struct RuleTable(Box<[u32]>);

    struct Universe {
        p01: TMap,
        p10: TMap,
        active: TSet,
        generation: u64,
        rule_table: RuleTable,
        x: i64,
        y: i64,
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
            let mut table = vec![0; 65512];

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
            let x = 0;
            let y = 0;

            Universe {
                p01, p10, active, generation, rule_table, x, y
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

        fn set_cell(&mut self, mut x: i64, mut y: i64) {
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
    
        pub fn render(&self, gl: &mut GL, canvas: &mut HtmlCanvasElement, 
            program: &WebGlProgram, position_attribute_location: u32, position_buffer: &WebGlBuffer, resolution_uniform_location: &WebGlUniformLocation) {
            self.resize_gl(gl, canvas);
            gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
            
            gl.clear_color(0_f32, 0_f32, 0_f32, 0_f32);
            gl.clear(GL::COLOR_BUFFER_BIT);

            gl.use_program(Some(program));

              // Turn on the attribute
            gl.enable_vertex_attrib_array(position_attribute_location);

            // Bind the position buffer.
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(position_buffer));

            // Tell the attribute how to get data out of positionBuffer (ARRAY_BUFFER)
            let size = 2;          // 2 components per iteration
            let gl_type = GL::FLOAT;   // the data is 32bit floats
            let normalize = false; // don't normalize the data
            let stride = 0;        // 0 = move forward size * sizeof(type) each iteration to get the next position
            let offset = 0;        // start at the beginning of the buffer
            gl.vertex_attrib_pointer_with_i32(
                position_attribute_location, size, gl_type, normalize, stride, offset);

            // set the resolution
            gl.uniform2f(Some(resolution_uniform_location), canvas.width() as f32, canvas.height() as f32);

            // draw 50 random rectangles in random colors

            //unimplemented!("not implemented");

        }

        fn resize_gl(&self, gl: &mut GL, canvas: &mut HtmlCanvasElement) {
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

        fn move_view(&mut self, x: i64, y: i64) {
            self.x += x;
            self.y += y;
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
}   */