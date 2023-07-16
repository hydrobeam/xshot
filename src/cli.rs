use clap::{Parser, ValueEnum};
use image::ImageOutputFormat;

#[derive(Parser, Debug)]
#[command(name = "xshot")]
#[command(author = "Laith Bahodi <laithbahodi@gmail.com>")]
#[command(about = "The XS screenshot tool for X11")]
#[command(author, version, about, long_about=None)]
pub struct Cli {
    /// The window name to target.
    ///
    /// Queries the _NET_WM_NAME property (i.e. the full display name of the window):
    ///
    /// https://specifications.freedesktop.org/wm-spec/1.3/ar01s05.html
    ///
    /// Matching is fuzzy, a screenshot will be taken of the first match.
    #[arg(short, long, conflicts_with_all = ["class", "wid"])]
    pub name: Option<String>,

    /// The window class to target.
    ///
    /// Queries the WM_CLASS property (i.e. the overall class of application: "Emacs", "Firefox", ...):
    ///
    /// https://tronche.com/gui/x/icccm/sec-4.html#WM_CLASS
    ///
    /// Matching is fuzzy, a screenshot will be taken of the first match.
    #[arg(short, long, conflicts_with_all=["wid", "name"])]
    pub class: Option<String>,

    /// The specific XID of a window to target.
    ///
    /// Can use a tool like `xwininfo` to find the ID of a particular window.
    #[arg(long, conflicts_with_all=["class", "name"])]
    pub wid: Option<String>,

    /// The topleft corner of the screenshot
    #[arg(short, long, num_args(2), default_values=["0", "0"])]
    pub position: Vec<i16>,

    /// Size of the screenshot.
    ///
    /// Defaults to the dimensions of the target window.
    #[arg(short, long, num_args(2))]
    pub size: Option<Vec<u16>>,

    /// The image format for the screenshot
    #[arg(short, long, value_enum, default_value = "jpeg")]
    pub format: OutputFormat,

    /// How long to wait before capturing the screenshot, in seconds.
    ///
    /// Accepts a float: `--delay 4.5` will wait 4.5 seconds.
    #[arg(short, long)]
    pub delay: Option<f64>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Png,
    Jpg,
    Jpeg,
    Gif,
    Bmp,
}

impl From<OutputFormat> for ImageOutputFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Png => ImageOutputFormat::Png,
            OutputFormat::Jpeg => ImageOutputFormat::Jpeg(50),
            OutputFormat::Jpg => ImageOutputFormat::Jpeg(50),
            OutputFormat::Gif => ImageOutputFormat::Gif,
            OutputFormat::Bmp => ImageOutputFormat::Bmp,
        }
    }
}

impl OutputFormat {
    /// Convert formats to mime types for writing to clipboard
    // NOTE: found out that mime types work from reading:
    // https://github.com/edrosten/x_clipboard/blob/master/selection.cc#L287-L289
    pub fn to_mime_type(self) -> &'static [u8] {
        match self {
            OutputFormat::Png => b"image/png",
            OutputFormat::Jpg | OutputFormat::Jpeg => b"image/jpeg",
            OutputFormat::Gif => b"image/gif",
            OutputFormat::Bmp => b"image/bmp",
        }
    }
}
