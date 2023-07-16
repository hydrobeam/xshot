# XSshot

The XS screenshot tool for X11!

Grabs screenshots of specific windows (or your whole screen) in the format of your choice and writes them to your clipboard or `stdout`. That's all.

## Install

`xsshot` is available on [`crates.io`](https://example.com)

```
cargo install xsshot
```

## Usage

With no arguments, `xsshot` will simply screenshot your current screen and copy it to your clipboard:

```bash
xsshot
```

Takes a screenshot of the window whose name contains "emacs" as a `.jpeg`:

```bash
xsshot -n emacs -f jpeg
```

`xsshot` recognizes when it's in a pipe and redirects its output to `stdout` instead of the clipboard:

```bash
xsshot -f bmp -c firefox > "firefox.bmp"
```

#### Output of `xsshot -h`

```man
The XS screenshot tool for X

Usage: xsshot [OPTIONS]

Options:
  -n, --name <NAME>                     The window name to target
  -c, --class <CLASS>                   The window class to target. Incomptabile with `name`
  -p, --position <POSITION> <POSITION>  The topleft corner of the screenshot [default: 0 0]
  -s, --size <SIZE> <SIZE>              Size of the screenshot
  -f, --format <FORMAT>                 The image format for the screenshot [default: png] [possible values: png, jpg, jpeg, gif, bmp]
  -h, --help                            Print help (see more with '--help')
  -V, --version                         Print version
```

## Dependencies

- `clap`: easy way to setup an excellent CLI
- `xcb`: bindings to `xcb`, required for reading screen/window data + writing to clipboard
- `image`: converting image data to convenient formats

## Name

Take your pick:

- Extra Small {XS} Screen(SHOT) => `xsshot`
- (XS)erver Screen(SHOT) => `xsshot`
