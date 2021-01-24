use serde::{Serialize,Deserialize};
#[derive(Debug,Deserialize,Serialize)]
pub struct Puzzle{
    pub rows:Vec<Vec<usize>>,
    pub columns:Vec<Vec<usize>>,
}
impl Puzzle{
    pub fn dimensions(&self)->PuzzleDimensions{
        PuzzleDimensions{
            width:self.columns.len(),
            height:self.rows.len(),
        }
    }
}
#[derive(Debug,Serialize)]
pub struct PuzzleDimensions{
    width:usize,
    height:usize,
}