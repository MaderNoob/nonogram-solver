mod puzzle;
mod solver;
use std::{collections::HashMap, fs::File};

use puzzle::Puzzle;
use solver::PuzzleSolver;
use websocket::{sync::Server, Message};
fn main() {
    println!("Waiting for client");
    let file = File::open("puzzles.json").unwrap();
    let puzzles: HashMap<String, Puzzle> = serde_json::from_reader(&file).unwrap();
    let puzzle = &puzzles["QR"];

    let mut server = Server::bind("127.0.0.1:4741").unwrap();
    let mut client = server.accept().unwrap().accept().unwrap();
    let message_string = serde_json::to_string(&puzzle.dimensions()).unwrap();
    let message = Message::text(message_string);
    client.send_message(&message).unwrap();

    let mut solver=PuzzleSolver::new(client,puzzle);
    solver.solve();
}
