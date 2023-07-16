mod cli;
mod x_interface;

use std::io::{stdout, Cursor, IsTerminal, Write};

use clap::Parser;
use image::RgbaImage;
use x_interface::XInterface;

fn main() -> xcb::Result<()> {
    let cli = cli::Cli::parse();

    let (conn, screen_num) = xcb::Connection::connect(None)?;
    let x_handle = XInterface::new(&conn, screen_num as usize);

    let ret_img: RgbaImage =
        x_handle.establish_image(cli.window.as_deref(), cli.position, cli.size)?;

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
