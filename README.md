<div align="center">

  <a href="https://github.com/orhun/runst">
    <img src="assets/runst-logo.jpg" width="300">
  </a>

#### **`runst`** — A dead simple notification daemon 🔔💬

</div>

[Desktop notifications](https://wiki.archlinux.org/title/Desktop_notifications) are small, passive popup dialogs that notify the user of particular events in an asynchronous manner. These passive popups can automatically disappear after a short period of time.

`runst` is the server implementation of [freedesktop.org](https://www.freedesktop.org/wiki) - [Desktop Notifications Specification](https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html) and it can be used to receive notifications from applications via [D-Bus](https://www.freedesktop.org/wiki/Software/dbus/). As of now, only [X11](https://en.wikipedia.org/wiki/X_Window_System) is supported.

<div align="center">

  <a href="https://github.com/orhun/runst">
    <img src="assets/runst-demo.gif">
  </a>

</div>

## Features

- Fully customizable notification window (size, location, text, colors).
- Template-powered ([Jinja2](http://jinja.pocoo.org/)/[Django](https://docs.djangoproject.com/en/3.1/topics/templates/)) notification text.
- Auto-clear notifications based on a fixed time or estimated read time.
- Run custom OS commands based on the matched notifications.

## Installation

### From crates.io

`runst` can be installed from [crates.io](https://crates.io/crates/runst):

```sh
cargo install runst
```

Minimum supported Rust version is `1.63.0`.

### Binary releases

See the available binaries for different operating systems/architectures from the [releases page](https://github.com/orhun/runst/releases).

\* Release tarballs are signed with the following PGP key: [AEF8C7261F4CEB41A448CBC41B250A9F78535D1A](https://keyserver.ubuntu.com/pks/lookup?search=0x1B250A9F78535D1A&op=vindex)

### Build from source

#### Prerequisites

- [D-Bus](https://www.freedesktop.org/wiki/Software/dbus)
- [GLib](https://wiki.gnome.org/Projects/GLib)
- [Pango](https://pango.gnome.org)

#### Instructions

1. Clone the repository.

```sh
git clone https://github.com/orhun/runst && cd runst/
```

2. Build.

```sh
CARGO_TARGET_DIR=target cargo build --release
```

Binary will be located at `target/release/runst`.

## Usage

## Configuration

- context

## Why this exists?

I have been an user of [dunst](https://github.com/dunst-project/dunst) for a long time. However, they made some [uncool breaking changes](https://github.com/dunst-project/dunst/issues/940) in [v1.7.0](https://github.com/dunst-project/dunst/releases/tag/v1.7.0) and it completely broke my configuration. That day, I refused to update `dunst` (I was too lazy to re-configure) and decided to write my own notification server using Rust.

I wanted to keep `runst` simple since the way I use `dunst` was really simple. I was only showing an overlay window on top of [i3status](https://github.com/i3/i3status) as shown below:

![runst use case](assets/runst-demo2.gif)

And that's how `runst` is born.

## Similar projects

- [wired-notify](https://github.com/Toqozz/wired-notify)

## License

Licensed under either of [Apache License Version 2.0](http://www.apache.org/licenses/LICENSE-2.0) or [The MIT License](http://opensource.org/licenses/MIT) at your option.

## Copyright

Copyright © 2022, [Orhun Parmaksız](mailto:orhunparmaksiz@gmail.com)
