#![allow(non_snake_case)]

mod app;
mod command;
mod entities;
mod io;
mod linetypes;
mod modules;
mod patterns;
mod scene;
mod snap;
mod ui;

fn main() -> iced::Result {
    app::run()
}
