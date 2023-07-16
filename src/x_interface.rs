use std::cell::OnceCell;

use image::RgbaImage;
use xcb::{x, Result};

use crate::cli::OutputFormat;

macro_rules! atoms {
    ($($item: ident,)*) => {
        struct Atoms<'a> {
            connection: &'a xcb::Connection,
            $($item: OnceCell<x::Atom>,)*
        }

        impl<'a> Atoms<'a> {
            fn new(connection: &'a xcb::Connection) -> Self {
                Self {
                    connection,
                    $($item: OnceCell::new(),)*
                }
            }

            $(atom!($item);)*
        }
    };
}

macro_rules! atom {
    ($name: ident) => {
        fn $name(&self) -> x::Atom {
            *self.$name.get_or_init(|| {
                self.connection
                    .wait_for_reply(self.connection.send_request(&x::InternAtom {
                        only_if_exists: false,
                        name: stringify!($name).to_uppercase().as_bytes(),
                    }))
                    .unwrap()
                    .atom()
            })
        }
    };
}

atoms!(
    _net_client_list,
    utf8_string,
    _net_wm_name,
    clipboard,
    targets,
);
/// self.atoms.net_client_list()

pub struct XInterface<'a> {
    connection: &'a xcb::Connection,
    screen: x::Window,
    atoms: Atoms<'a>,
}

impl<'a> XInterface<'a> {
    pub fn new(connection: &'a xcb::Connection, screen_num: usize) -> Self {
        let setup: &x::Setup = connection.get_setup();
        let screen = setup
            .roots()
            .nth(screen_num as usize)
            .expect("invalid x display")
            .root();

        Self {
            connection: &connection,
            screen,
            atoms: Atoms::new(connection),
        }
    }

    pub fn establish_image(
        &self,
        window_name: Option<&str>,
        position: Vec<i16>,
        size: Option<Vec<u16>>,
    ) -> Result<RgbaImage> {
        let wid = if let Some(name) = window_name {
            self.find_window_class(name)?
        } else {
            self.screen
        };

        let size = if let Some(size) = size {
            [size[0], size[1]]
        } else {
            self.calc_geometry(wid)?
        };

        let window_image = self.connection.send_request(&xcb::x::GetImage {
            format: x::ImageFormat::ZPixmap,
            drawable: x::Drawable::Window(wid),
            x: position[0],
            y: position[1],
            width: size[0],
            height: size[1],
            plane_mask: std::u32::MAX,
        });

        let window_image = self.connection.wait_for_reply(window_image)?;
        let img_data = window_image.data();
        let mut pixels = Vec::with_capacity(img_data.len());
        // convert data from BGRx to RGBA
        for chunk in img_data.chunks(4) {
            pixels.push(chunk[2]); // R
            pixels.push(chunk[1]); // G
            pixels.push(chunk[0]); // B
            pixels.push(0xff); // A (actually chunk[3], but it's always 0)
        }
        Ok(
            image::RgbaImage::from_raw(size[0].into(), size[1].into(), pixels)
                .expect("failed image conversion"),
        )
    }

    /// Search for a window class which contains `name`.
    fn find_window_class(&self, name: &str) -> Result<x::Window> {
        let client_list = self.connection.send_request(&x::GetProperty {
            delete: false,
            window: self.screen,
            property: self.atoms._net_client_list(),
            r#type: x::ATOM_NONE,
            long_offset: 0,
            long_length: 1024,
        });
        let list = self.connection.wait_for_reply(client_list)?;

        for client in list.value() {
            let cookie = self.connection.send_request(&x::GetProperty {
                delete: false,
                window: *client,
                property: self.atoms._net_wm_name(),
                r#type: self.atoms.utf8_string(),
                long_offset: 0,
                long_length: 1024,
            });
            let reply = self.connection.wait_for_reply(cookie)?;
            let title = reply.value();
            let title = std::str::from_utf8(title).expect("invalid utf8");
            // dbg!(title);
            if title.to_lowercase().contains(&name.to_lowercase()) {
                return Ok(*client);
            }
        }

        panic!("not able to find window class")
    }

    fn calc_geometry(&self, wid: x::Window) -> Result<[u16; 2]> {
        let window_geom = self.connection.send_request(&x::GetGeometry {
            drawable: x::Drawable::Window(wid),
        });

        let window_geom = self.connection.wait_for_reply(window_geom)?;

        Ok([window_geom.width(), window_geom.height()])
    }
}

/// Query the x-server to get iamge data.
///
/// Depending on whether window_name/size are not None, we also query for
/// - additional windows matching the passed name
/// - size of the window to be screenshotted

impl XInterface<'_> {
    pub fn write_to_clipboard(&self, img_buf: &[u8], format: OutputFormat) -> Result<()> {
        let window = self.connection.generate_id();
        self.connection.send_and_check_request(&x::CreateWindow {
            depth: x::COPY_FROM_PARENT as u8,
            wid: window,
            parent: self.screen,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            border_width: 0,
            class: x::WindowClass::InputOnly,
            visual: x::COPY_FROM_PARENT,
            value_list: &[],
        })?;

        let image_format = self
            .connection
            .wait_for_reply(self.connection.send_request(&x::InternAtom {
                only_if_exists: true,
                name: format.to_mime_type(),
            }))?
            .atom();

        self.connection
            .send_and_check_request(&x::SetSelectionOwner {
                owner: window,
                selection: self.atoms.clipboard(),
                time: x::CURRENT_TIME,
            })?;

        let got_select = self.connection.send_request(&x::GetSelectionOwner {
            selection: self.atoms.clipboard(),
        });
        if self.connection.wait_for_reply(got_select)?.owner() != window {
            panic!("unable to establish window as clipboard owner")
        }

        loop {
            let event = self.connection.wait_for_event()?;
            let mut escape = false;

            match event {
                xcb::Event::X(event) => match event {
                    x::Event::SelectionClear(_) => {
                        break;
                    }
                    x::Event::SelectionRequest(event) => {
                        if event.target() == image_format {
                            self.connection.send_and_check_request(&x::ChangeProperty {
                                mode: x::PropMode::Replace,
                                window: event.requestor(),
                                property: event.property(),
                                r#type: event.target(),
                                data: img_buf,
                            })?;
                            // give up ownership of clipboard by destroying the window, we're done.
                            // https://tronche.com/gui/x/icccm/sec-2.html
                            //
                            // Alternatively, the client may destroy the window
                            // used as the owner value of the SetSelectionOwner request,
                            // or the client may terminate. In both cases, the ownership
                            // of the selection involved will revert to None .
                            self.connection
                                .send_and_check_request(&x::DestroyWindow { window })?;
                            escape = true;
                        } else if event.target() == self.atoms.targets() {
                            self.connection.send_request(&x::ChangeProperty {
                                mode: x::PropMode::Replace,
                                window: event.requestor(),
                                property: event.property(),
                                r#type: x::ATOM_ATOM,
                                data: &[image_format],
                            });
                        }

                        self.connection.send_request(&x::SendEvent {
                            propagate: false,
                            destination: x::SendEventDest::Window(event.requestor()),
                            event_mask: x::EventMask::empty(),
                            event: &x::SelectionNotifyEvent::new(
                                event.time(),
                                event.requestor(),
                                event.selection(),
                                event.target(),
                                event.property(),
                            ),
                        });
                        self.connection.flush()?;
                        if escape {
                            break;
                        }
                    }
                    _ => {}
                },
                xcb::Event::Unknown(_) => unreachable!(),
            }
        }
        Ok(())
    }
}
