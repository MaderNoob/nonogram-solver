use core::panic;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

use array2d::Array2D;
use serde::__private::ser;
use websocket::{
    server::NoTlsAcceptor,
    sync::{Client, Server},
    Message,
};

use crate::puzzle::Puzzle;

const SLEEP_TIME: u64 = 1000;

#[derive(Debug, Clone)]
pub struct ColumnBlockState {
    size: usize,
    index_in_block: Option<usize>,
}
impl ColumnBlockState {
    pub fn new() -> ColumnBlockState {
        ColumnBlockState {
            size: 0,
            index_in_block: None,
        }
    }
    // pub fn space(&self) -> bool {
    //     match self.index_in_block {
    //         Some(i) => i + 1 == self.size,
    //         None => true,
    //     }
    // }
    // pub fn block(&mut self) -> bool {
    //     match &mut self.index_in_block {
    //         Some(i) => {
    //             if *i + 1 < self.size {
    //                 *i += 1;
    //                 true
    //             } else {
    //                 false
    //             }
    //         }
    //         None => panic!(),
    //     }
    // }
}

#[derive(Debug, Clone)]
pub struct ColumnState {
    blocks: Vec<usize>,
    size: usize,
    total_blocks: usize,
    index_in_column: usize,
    block_number: usize,
    block_state: ColumnBlockState,
    spaces_after_each_block: Vec<usize>,
}
impl ColumnState {
    fn spaces_after_cur_block_mut(&mut self) -> &mut usize {
        &mut self.spaces_after_each_block[self.block_number]
    }
    fn do_space(&mut self) -> bool {
        match self.block_state.index_in_block {
            Some(i) => {
                if i + 1 == self.block_state.size {
                    self.block_state.index_in_block = None;
                    *self.spaces_after_cur_block_mut() += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                *self.spaces_after_cur_block_mut() += 1;
                true
            }
        }
    }
    pub fn space(&mut self) -> bool {
        if self.do_space() {
            self.next_col()
        } else {
            false
        }
    }
    fn do_block(&mut self) -> bool {
        match &mut self.block_state.index_in_block {
            None => {
                if *self.spaces_after_cur_block_mut() == 0 && self.block_number > 0 {
                    false
                } else {
                    // if this is the last block
                    if self.block_number == self.total_blocks {
                        false
                    } else {
                        self.block_number += 1;
                        self.load_block_size();
                        self.block_state.index_in_block = Some(0);
                        true
                    }
                }
            }
            Some(i) => {
                if *i + 1 < self.block_state.size {
                    *i += 1;
                    true
                } else {
                    false
                }
            }
        }
    }
    pub fn block(&mut self) -> bool {
        if self.do_block() {
            self.next_col()
        } else {
            false
        }
    }
    fn next_col(&mut self) -> bool {
        self.index_in_column += 1;
        if self.index_in_column > self.size {
            self.index_in_column -= 1;
            false
        } else {
            true
        }
    }
    fn load_block_size(&mut self) {
        if self.block_number == 0 {
            self.block_state.size = 0;
        } else {
            self.block_state.size = self.blocks[self.block_number - 1];
        }
    }
    fn back(&mut self) {
        self.index_in_column -= 1;
        match &mut self.block_state.index_in_block {
            Some(i) => {
                if *i == 0 {
                    self.block_number -= 1;
                    self.load_block_size();
                    self.block_state.index_in_block = None;
                } else {
                    *i -= 1
                }
            }
            None => {
                let spaces = self.spaces_after_cur_block_mut();
                *spaces -= 1;
                // if this was the last space, reload the block
                if *spaces == 0 && self.block_number > 0 {
                    self.block_state.index_in_block = Some(self.block_state.size - 1);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct RowState {
    index: usize,
    total_blocks: usize,
    block_size: usize,
    block_index: usize,
    required_space: usize,
    index_in_row: usize,
}
impl RowState {
    fn calculate_required_space(row: &[usize]) -> usize {
        row.iter().sum::<usize>() + row.len() - 1
    }
    pub fn new(row: &[usize], index: usize, required_space: usize) -> RowState {
        RowState {
            block_size: row[0],
            total_blocks: row.len(),
            block_index: 0,
            index_in_row: 0,
            required_space,
            index,
        }
    }
}

#[derive(serde::Serialize)]
struct Command {
    row: usize,
    column: usize,
    draw: bool,
}

pub struct PuzzleSolver {
    columns: Vec<ColumnState>,
    rows: Vec<Vec<usize>>,
    total_rows: usize,
    row_size: usize,
    row: RowState,
    required_space_for_each_row: Vec<usize>,
    board: Array2D<bool>,
    connection: Client<TcpStream>,
}
impl PuzzleSolver {
    pub fn new(connection: Client<TcpStream>, puzzle: &Puzzle) -> PuzzleSolver {
        let required_space_for_each_row: Vec<usize> = puzzle
            .rows
            .iter()
            .map(|r| RowState::calculate_required_space(r.as_slice()))
            .collect();
        PuzzleSolver {
            columns: puzzle
                .columns
                .iter()
                .map(|col| ColumnState {
                    blocks: col.clone(),
                    size: puzzle.rows.len(),
                    total_blocks: col.len(),
                    index_in_column: 0,
                    block_number: 0,
                    block_state: ColumnBlockState {
                        size: 0,
                        index_in_block: None,
                    },
                    spaces_after_each_block: vec![0; col.len() + 1],
                })
                .collect(),
            rows: puzzle.rows.clone(),
            row_size: puzzle.columns.len(),
            row: RowState::new(&puzzle.rows[0], 0, required_space_for_each_row[0]),
            required_space_for_each_row,
            board: Array2D::filled_with(false, puzzle.rows.len(), puzzle.columns.len()),
            connection,
            total_rows: puzzle.rows.len(),
        }
    }
    pub fn solve(&mut self) {
        self.solve_block();
    }
    fn is_last_block_in_row(&self) -> bool {
        self.row.block_index + 1 == self.row.total_blocks
    }
    // calculate the current block's required space including its preceeding space
    fn current_blocks_required_space(&self) -> usize {
        if self.is_last_block_in_row() {
            self.row.block_size
        } else {
            self.row.block_size + 1
        }
    }
    pub fn solve_block(&mut self) {
        self.row.required_space -= self.current_blocks_required_space();
        let max_start_index =
            self.row_size - self.row.required_space - self.current_blocks_required_space();
        self.try_start_indexes(max_start_index);
        self.row.required_space += self.current_blocks_required_space();
    }
    fn try_start_indexes(&mut self, max_start_index: usize) {
        for start_index in self.row.index_in_row..=max_start_index {
            if self.draw_block(start_index) {
                if !self.is_last_block_in_row()
                    && !self.columns[start_index + self.row.block_size].space()
                {
                    // println!(
                    //     "Failed to space column {}",
                    //     start_index + self.row.block_size
                    // );
                    // self.print_columns_board();
                    // let mut buf = [0u8; 1];
                    // std::io::stdin().read(&mut buf);
                    // // cleanup
                    // for cleanup_col in self.row.index_in_row..start_index {
                    //     self.columns[cleanup_col].back();
                    //     self.print_columns_board();
                    // }
                    self.print_columns_board();
                    self.undraw_block(start_index);
                    self.print_columns_board();
                }else{
                    let spaces_amount = start_index - self.row.index_in_row;
                    self.row.index_in_row += self.current_blocks_required_space() + spaces_amount;
                    self.solve_next_block();
                    self.row.index_in_row -= self.current_blocks_required_space() + spaces_amount;
                    if !self.is_last_block_in_row() {
                        self.columns[start_index + self.row.block_size].back();
                        self.print_columns_board();
                    }
                    self.undraw_block(start_index);
                }
            }
            if !self.columns[start_index].space() {
                // cleanup
                for cleanup_col in self.row.index_in_row..start_index {
                    self.columns[cleanup_col].back();
                    self.print_columns_board();
                }
                return;
            }
            self.print_columns_board();
        }
        for cleanup_col in self.row.index_in_row..=max_start_index {
            self.columns[cleanup_col].back();
            self.print_columns_board();
        }
    }
    fn solve_next_block(&mut self) {
        if self.row.block_index + 1 >= self.row.total_blocks {
            if self.row.index + 1 == self.total_rows {
                println!("Done!!");
                std::process::exit(0);
            } else {
                // move to the next row
                if self.fill_rest_of_row_with_spaces() {
                    let old_row_state = self.row.clone();
                    let new_row_index = self.row.index + 1;
                    self.row = RowState::new(
                        &self.rows[new_row_index],
                        new_row_index,
                        self.required_space_for_each_row[new_row_index],
                    );
                    self.solve_block();
                    self.row = old_row_state;
                    self.cleanup_end_of_row_spaces();
                }
            }
        } else {
            self.row.block_index += 1;
            self.row.block_size = self.rows[self.row.index][self.row.block_index];
            self.solve_block();
            self.row.block_index -= 1;
            self.row.block_size = self.rows[self.row.index][self.row.block_index];
        }
    }
    fn fill_rest_of_row_with_spaces(&mut self) -> bool {
        for col in self.row.index_in_row..self.row_size {
            if !self.columns[col].space() {
                // cleanup
                for cleanup_col in self.row.index_in_row..col {
                    self.columns[cleanup_col].back();
                    self.print_columns_board();
                }
                return false;
            }
            self.print_columns_board();
        }
        true
    }
    fn cleanup_end_of_row_spaces(&mut self) {
        for col in self.row.index_in_row..self.row_size {
            self.columns[col].back();
            self.print_columns_board();
        }
    }
    fn draw_block(&mut self, start_index: usize) -> bool {
        for col in start_index..start_index + self.row.block_size {
            if self.columns[col].block() {
                self.board[(self.row.index, col)] = true;
                self.send_command(self.row.index, col, true);
            } else {
                for cleanup_col in start_index..col {
                    self.board[(self.row.index, cleanup_col)] = false;
                    self.columns[cleanup_col].back();
                    self.send_command(self.row.index, cleanup_col, false);
                }
                return false;
            }
        }
        true
    }
    fn undraw_block(&mut self, start_index: usize) {
        for col in start_index..start_index + self.row.block_size {
            self.columns[col].back();
            self.board[(self.row.index, col)] = false;
            self.send_command(self.row.index, col, false);
        }
    }
    fn send_command(&mut self, row: usize, column: usize, draw: bool) {
        let message_string = serde_json::to_string(&Command { row, column, draw }).unwrap();
        let message = Message::text(&message_string);
        self.connection.send_message(&message).unwrap();
        self.print_columns_board();
        // let mut buf = [0u8; 1];
        // std::io::stdin().read(&mut buf);
        std::thread::sleep(std::time::Duration::from_micros(SLEEP_TIME));
    }
    fn construct_board_from_columns(&self) -> Array2D<char> {
        let mut board = Array2D::filled_with('?', self.rows.len(), self.columns.len());
        let mut columns = self.columns.clone();
        for (col_index, col) in columns.iter_mut().enumerate() {
            // got to the start of the block
            loop {
                if col.index_in_column == 0 {
                    break;
                }
                board[(col.index_in_column - 1, col_index)] =
                    if col.block_state.index_in_block.is_some() {
                        '#'
                    } else {
                        '.'
                    };
                col.back();
            }
        }
        board
    }
    fn print_columns_board(&self) {
        // let board = self.construct_board_from_columns();
        // for row in 0..self.rows.len() {
        //     for col in 0..self.columns.len() {
        //         print!("{}", board[(row, col)]);
        //     }
        //     println!();
        // }
        // println!();
    }
}
