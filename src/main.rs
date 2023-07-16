mod cli;
mod types;
mod x_interface;

use std::io::{stdout, Cursor, IsTerminal, Write};

use clap::Parser;
use image::RgbaImage;
use x_interface::XInterface;

use types::WindowTarget;

fn main() -> xcb::Result<()> {
    let cli = cli::Cli::parse();

    let window_query = if let Some(i) = &cli.name {
        Some(WindowTarget::Name(i))
    } else if let Some(i) = &cli.class {
        Some(WindowTarget::Class(i))
    } else if let Some(i) = &cli.wid {
        Some(WindowTarget::Wid(i))
    } else {
        None
    };

    let (conn, screen_num) = xcb::Connection::connect(None)?;
    let x_handle = XInterface::new(&conn, screen_num as usize);

    let ret_img: RgbaImage = x_handle.establish_image(window_query, cli.position, cli.size)?;

    let mut cursor = Cursor::new(Vec::new());
    ret_img.write_to(&mut cursor, cli.format).unwrap();

    let mut io_out = stdout().lock();
    // if we're in a terminal, copy to clipboard
    // otherwise, just write to sdout
    if io_out.is_terminal() {
        x_handle
            .write_to_clipboard(&cursor.into_inner(), cli.format)
            .expect("failed writing to clipboard");
    } else {
        io_out
            .write_all(&cursor.into_inner())
            .expect("failed writing to stdout");
    }
    Ok(())
}
