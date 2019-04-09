extern crate cfg_if;
extern crate js_sys;
extern crate wasm_bindgen;

mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Dead = 0,
    Alive = 1,
}

impl Cell {
    fn toggle(&mut self) {
        *self = match *self {
            Cell::Dead => Cell::Alive,
            Cell::Alive => Cell::Dead,
        };
    }
}

#[wasm_bindgen]
#[derive(Debug, PartialEq, Eq)]
pub struct Universe {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
}

#[wasm_bindgen]
impl Universe {

    pub fn new(width: u32, height: u32) -> Universe {

        let cells = (0..width * height)
            .map(|_i| Cell::Dead)
            .collect();

        Universe {
            width,
            height,
            cells,
        }
    }

    pub fn randomize(&mut self) {
        self.cells = (0..self.width * self.height)
            .map(|_i| {
                if js_sys::Math::random() > 0.5 {
                    Cell::Alive
                } else {
                    Cell::Dead
                }
            }).collect();
    }

    pub fn reset(&mut self) {
        self.cells = (0..self.width * self.height).map(|_i| Cell::Dead).collect();
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = width;
        self.reset();
    }

    pub fn set_height(&mut self, height: u32) {
        self.height = height;
        self.reset();
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn cells(&self) -> *const Cell {
        self.cells.as_ptr()
    }

    // Decided to make the wrapping happen in here
    fn get_index(&self, row: u32, column: u32) -> usize {
        let wrapped_row = row % self.height;
        let wrapped_column = column % self.width;
        (wrapped_row * self.width + wrapped_column) as usize
    }

    // Implemented as a wrapping universe around the edges
    fn live_neighbor_count(&self, row: u32, column: u32) -> u8 {
        let mut count = 0;
        for delta_row in [self.height - 1, 0, 1].iter().cloned() {
            for delta_col in [self.width - 1, 0, 1].iter().cloned() {
                if delta_row == 0 && delta_col == 0 {
                    continue;
                }

                let neighbor_row = row + delta_row;
                let neighbor_col = column + delta_col;
                let index = self.get_index(neighbor_row, neighbor_col);
                count += self.cells[index] as u8;
            }
        }
        count
    }


    pub fn tick(&mut self) {
        let mut next = self.cells.clone();

        for row in 0..self.height {
            for col in 0..self.width {
                let index = self.get_index(row, col);
                let cell = self.cells[index];
                let live_neighbors = self.live_neighbor_count(row, col);

                let next_cell = match (cell, live_neighbors) {
                    (Cell::Alive, x) if x < 2 => Cell::Dead,
                    (Cell::Alive, 2) | (Cell::Alive, 3) => Cell::Alive,
                    (Cell::Alive, x) if x > 3 => Cell::Dead,
                    (Cell::Dead, 3) => Cell::Alive,
                    (otherwise, _) => otherwise,
                };

                next[index] = next_cell;
            }
        }

        self.cells = next;
    }

    pub fn toggle_cell(&mut self, row: u32, column: u32) {
        let idx = self.get_index(row, column);
        self.cells[idx].toggle();
    }
}

impl Universe {

    pub fn get_cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn set_cells(&mut self, cells: &[(u32, u32)]) {
        for (row, col) in cells.iter().cloned() {
            let idx = self.get_index(row, col);
            self.cells[idx] = Cell::Alive;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_cell_toggle() {
        let mut cell = Cell::Dead;
        cell.toggle();
        assert_eq!(cell, Cell::Alive);
        cell.toggle();        
        assert_eq!(cell, Cell::Dead);        
    }

    #[test]
    fn test_universe_size() {
        let universe = Universe::new(2, 10);
        assert_eq!(universe.cells.len(), 20);
    }

    #[test]
    fn test_wrapping_index() {
        let mut universe = Universe::new(2, 3);
        universe.toggle_cell(3, 0);

        let mut expected = Universe::new(2, 3);
        expected.toggle_cell(0, 0);
        assert_eq!(universe, expected);
    }
}
