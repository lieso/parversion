use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::hash::Hash;

struct Matrix<T> {
    data: Vec<Vec<T>>,
}

impl<T: Default + Clone> Matrix<T> {
    fn new(rows: usize, cols: usize) -> Matrix<T> {
        Matrix {
            data: vec![vec![T::default(); cols]; rows],
        }
    }

		fn add_row(&mut self, row: Vec<T>) {
			self.data.push(row);
		}

    fn set(&mut self, row: usize, col: usize, value: T) {
        self.data[row][col] = value;
    }

    fn get(&self, row: usize, col: usize) -> &T {
        &self.data[row][col]
    }
}

