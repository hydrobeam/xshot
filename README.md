# XShot

The XS screenshot tool for X11!

Grabs screenshots of specific windows (or your whole screen) in the format of your choice and writes them to your clipboard or `stdout`. That's all.

## Install

### Arch

`xshot` is on the AUR! It will also install the completions for `bash/zsh/fish` & the manpage.

```bash
yay -S xshot-git
```

## Usage

With no arguments, `xshot` will simply screenshot your current screen and copy it to your clipboard:

```bash
xshot
```

Takes a screenshot of the window whose name contains "emacs" as a `.png`:

```bash
xshot -n emacs -f png
```

`xshot` recognizes when it's in a pipe and redirects its output to `stdout` instead of the clipboard:

```bash
xshot -f bmp -c firefox > "firefox.bmp"
```

#### Output of `xshot -h`

```man
The XS screenshot tool for X11

Usage: xshot [OPTIONS]

Options:
  -n, --name <NAME>                     The window name to target
  -c, --class <CLASS>                   The window class to target
      --wid <WID>                       The specific XID of a window to target
  -p, --position <POSITION> <POSITION>  The topleft corner of the screenshot [default: 0 0]
  -s, --size <SIZE> <SIZE>              Size of the screenshot
  -f, --format <FORMAT>                 The image format for the screenshot [default: jpeg] [possible values: png, jpg, jpeg, gif, bmp]
  -d, --delay <DELAY>                   How long to wait before capturing the screenshot, in seconds
  -h, --help                            Print help (see more with '--help')
  -V, --version                         Print version
```

## Dependencies

- `clap`: easy way to setup an excellent CLI
- `xcb`: bindings to `xcb`, required for reading screen/window data + writing to clipboard
- `image`: converting image data to convenient formats

## Name

Take your pick:

- Extra Small {XS} Screen(SHOT) => `xshot`
- (XS)erver Screen(SHOT) => `xshot`
