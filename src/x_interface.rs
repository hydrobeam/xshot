use std::cell::OnceCell;

use image::RgbaImage;
use xcb::{x, Result};

use crate::WindowTarget;

use crate::cli::OutputFormat;

// Macros:
macro_rules! atoms {
    ($($item: ident,)*) => {
        /// Custom Lazily Implemented atoms
        ///
        /// Each individual atom is wrapped in a OnceCell that is initialized when it is used.
        /// (see the atom! macro)
        ///
        /// This avoids initializing every atom all at once when we setuo the connection,
        /// they are only established when they are first used.
        ///
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
                let atom_cookie = self.connection.send_request(&x::InternAtom {
                    only_if_exists: false,
                    // all names are pure ascii, this is fine
                    name: stringify!($name).to_ascii_uppercase().as_bytes(),
                });

                self.connection
                    .wait_for_reply(atom_cookie)
                    .expect("unable to establish atom")
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

/// Struct that executes all X-related operations
pub(crate) struct XInterface<'a> {
    /// The connection to x11
    connection: &'a xcb::Connection,
    /// Current screen
    screen: x::Window,
    /// Custom lazily-defined atoms
    ///
    /// # Example
    ///
    /// ```text
    /// // self.atoms.uft8_string is not defined here
    ///
    /// let atom = self.atoms.utf8_string(); // it is now defined
    /// self.atoms.uft8_string() // already defined, is not re-computed
    /// ```
    atoms: Atoms<'a>,
}

impl<'a> XInterface<'a> {
    pub fn new(connection: &'a xcb::Connection, screen_num: usize) -> Self {
        let setup: &x::Setup = connection.get_setup();
        let screen = setup
            .roots()
            .nth(screen_num)
            .expect("invalid x display")
            .root();

        Self {
            connection,
            screen,
            atoms: Atoms::new(connection),
        }
    }

    /// Wrapper around send_request + wait_for_reply to make it less verbose.
    pub fn request<R, C>(&self, r: &R) -> Result<C::Reply>
    where
        R: xcb::Request<Cookie = C>,
        C: xcb::CookieWithReplyChecked,
    {
        let cookie = self.connection.send_request(r);
        self.connection.wait_for_reply(cookie)
    }
}

impl<'a> XInterface<'a> {
    pub fn establish_image(
        &self,
        window_name: Option<WindowTarget>,
        position: Vec<i16>,
        size: Option<Vec<u16>>,
        delay: Option<f64>,
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

        if let Some(delay) = delay {
            let time = std::time::Duration::from_secs_f64(delay);
            eprintln!("Waiting {} seconds", time.as_secs_f64());
            std::thread::sleep(time)
        }

        let window_image = self.request(&xcb::x::GetImage {
            format: x::ImageFormat::ZPixmap,
            drawable: x::Drawable::Window(wid),
            x: position[0],
            y: position[1],
            width: size[0],
            height: size[1],
            plane_mask: std::u32::MAX,
        })?;

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
    fn find_window_class(&self, name: WindowTarget) -> Result<x::Window> {
        // returns all top level clients:
        // https://specifications.freedesktop.org/wm-spec/1.3/ar01s03.html
        let list = self.request(&x::GetProperty {
            delete: false,
            window: self.screen,
            property: self.atoms._net_client_list(),
            r#type: x::ATOM_NONE,
            long_offset: 0,
            long_length: 1024,
        })?;

        let (property, r#type, name) = match name {
            // use _NET_WM_NAME and not WM_NAME because WM_NAME just.. doesn't work?
            // it returns all blank strings.
            WindowTarget::Name(n) => (self.atoms._net_wm_name(), self.atoms.utf8_string(), n),
            // blank strings unless ATOM_STRING is used for the type
            WindowTarget::Class(n) => (x::ATOM_WM_CLASS, x::ATOM_STRING, n),
            WindowTarget::Wid(n) => {
                let res_id =
                    u32::from_str_radix(n.trim_start_matches("0x"), 16).expect("invalid window id");
                // SAFETY: there's no other way to get a window object,
                // so we need to trust the user and assume they're giving us a proper wid.
                return Ok(unsafe { std::mem::transmute::<u32, x::Window>(res_id) });
            }
        };
        for client in list.value() {
            let reply = self.request(&x::GetProperty {
                delete: false,
                window: *client,
                property,
                r#type,
                long_offset: 0,
                long_length: 1024,
            })?;
            let title = reply.value();
            let title = std::str::from_utf8(title).expect("invalid utf8 title");
            // Name search will always unwrap_or to get the original title

            // Class search is in the format: `\0FIRST\0SECOND\0`, SECOND has the contents we want
            // so we try to extract that.
            // https://tronche.com/gui/x/icccm/sec-4.html#WM_CLASS
            let title = title.split('\0').nth(1).unwrap_or(title);

            if title.to_lowercase().contains(&name.to_lowercase()) {
                eprintln!("matched against: {}", title);
                return Ok(*client);
            }
        }

        // TODO: proper erroring without panicking
        panic!("unable to find window class")
    }

    /// Gets the dimensions of the window to be screenshotted.
    fn calc_geometry(&self, wid: x::Window) -> Result<[u16; 2]> {
        let window_geom = self.connection.send_request(&x::GetGeometry {
            drawable: x::Drawable::Window(wid),
        });

        let window_geom = self.connection.wait_for_reply(window_geom)?;

        Ok([window_geom.width(), window_geom.height()])
    }
}

impl XInterface<'_> {
    /// Query the x-server to get image data.
    ///
    /// Depending on whether window_name/size are not None, we also query for
    /// - additional windows matching the passed name
    /// - size of the window to be screenshotted
    pub fn write_to_clipboard(&self, img_buf: &[u8], format: OutputFormat) -> Result<()> {
        let window = self.connection.generate_id();
        self.connection.send_and_check_request(&x::CreateWindow {
            // stolen directly from xcolor:
            // https://github.com/Soft/xcolor/blob/969d6525c4568a2fafd321fcd72a95481c5f3c7b/src/selection.rs#L88-L101
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

        // setup an atom for the mime type
        let image_format = self
            .request(&x::InternAtom {
                only_if_exists: true,
                name: format.to_mime_type(),
            })?
            .atom();

        // the overall process for writing to clipboard is described here:
        // https://tronche.com/gui/x/icccm/sec-2.html

        self.connection
            .send_and_check_request(&x::SetSelectionOwner {
                owner: window,
                selection: self.atoms.clipboard(),
                time: x::CURRENT_TIME,
            })?;

        // check if we succeeded in acquiring control of the selection
        if self
            .request(&x::GetSelectionOwner {
                selection: self.atoms.clipboard(),
            })?
            .owner()
            != window
        {
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
                        // targets is used by a caller to see which atoms we support
                        if event.target() == self.atoms.targets() {
                            self.connection.send_request(&x::ChangeProperty {
                                mode: x::PropMode::Replace,
                                window: event.requestor(),
                                property: event.property(),
                                r#type: x::ATOM_ATOM,
                                data: &[image_format],
                            });
                        } else if event.target() == image_format {
                            self.connection.send_and_check_request(&x::ChangeProperty {
                                mode: x::PropMode::Replace,
                                window: event.requestor(),
                                property: event.property(),
                                r#type: event.target(),
                                data: img_buf,
                            })?;
                            // give up ownership of clipboard by destroying the window,
                            // we've sent our data so we're done.
                            // https://tronche.com/gui/x/icccm/sec-2.html
                            //
                            // > Alternatively, the client may destroy the window
                            // > used as the owner value of the SetSelectionOwner request,
                            // > or the client may terminate. In both cases, the ownership
                            // > of the selection involved will revert to None .
                            self.connection
                                .send_and_check_request(&x::DestroyWindow { window })?;
                            // break out of the loop just before we send the last message
                            // REVIEW: what does the last message actually do
                            escape = true;
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
