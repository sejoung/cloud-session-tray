#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod desktop;
mod model;
mod profiles;
mod state;

fn main() {
    desktop::run();
}
