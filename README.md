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
$ cargo install runst
```

The minimum supported Rust version is `1.63.0`.

### Binary releases

See the available binaries for different operating systems/architectures from the [releases page](https://github.com/orhun/runst/releases).

Release tarballs are signed with the following PGP key: [AEF8C7261F4CEB41A448CBC41B250A9F78535D1A](https://keyserver.ubuntu.com/pks/lookup?search=0x1B250A9F78535D1A&op=vindex)

### Build from source

#### Prerequisites

- [D-Bus](https://www.freedesktop.org/wiki/Software/dbus)
- [GLib](https://wiki.gnome.org/Projects/GLib)
- [Pango](https://pango.gnome.org)

#### Instructions

1. Clone the repository.

```sh
$ git clone https://github.com/orhun/runst && cd runst/
```

2. Build.

```sh
$ CARGO_TARGET_DIR=target cargo build --release
```

Binary will be located at `target/release/runst`.

## Usage

### On Xorg startup

You can use [xinitrc](#xinitrc) or [xprofile](#xprofile) for autostarting `runst`.

#### xinitrc

If you are starting Xorg manually with [xinit](https://www.x.org/archive/X11R6.8.0/doc/xinit.1.html), you can `runst` on X server startup via [xinitrc](https://wiki.archlinux.org/title/Xinit#xinitrc):

`$HOME/.xinitrc`:

```sh
runst &
```

Long-running programs such as notification daemons should be started before the window manager, so they should either fork themself or be run in the background via appending `&` sign. Otherwise, the script would halt and wait for each program to exit before executing the window manager or desktop environment.

In the case of `runst` not being available since it's started at a faster manner than the window manager, you can add a delay as shown in the example below:

```sh
{ sleep 2; runst; } &
```

#### xprofile

If you are using a [display manager](https://wiki.archlinux.org/title/Display_manager), you can utilize an [xprofile](https://wiki.archlinux.org/title/Xprofile) file which allows you to execute commands at the beginning of the X user session.

The xprofile file, which is `~/.xprofile` or `/etc/xprofile`, can be styled similarly to [xinitrc](#xinitrc).

### As a D-Bus service

You can create a D-Bus service to launch `runst` automatically on the first notification action. For example, you can create the following service configuration:

`/usr/share/dbus-1/services/org.orhun.runst.service`:

```ini
[D-BUS Service]
Name=org.freedesktop.Notifications
Exec=/usr/bin/runst
```

Whenever an application sends a notification by sending a signal to `org.freedesktop.Notifications`, D-Bus activates `runst`.

Also, see [**#1**](https://github.com/orhun/runst/issues/1) for systemd integration.

## Configuration

`runst` configuration file supports [TOML](https://github.com/toml-lang/toml) format and the default configuration values can be found [here](./config/runst.toml).

If exists, configuration file is read from the following default locations:

- `$HOME/.config/runst/runst.toml`
- `$HOME/.runst/runst.toml`

You can also specify a path via `RUNST_CONFIG` environment variable.

### Global configuration

#### `log_verbosity`

Sets the [logging verbosity](https://docs.rs/log/latest/log/enum.Level.html). Possible values are `error`, `warn`, `info`, `debug` and `trace`.

#### `startup_notification`

Shows a notification at startup if set to `true`.

#### `geometry`

Sets the window geometry. The value format is `<width>x<height>+<x>+<y>`.

For setting this value, I recommend using a tool like [slop](https://github.com/naelstrof/slop) which helps with querying for a selection and printing the region to stdout.

#### `font`

Sets the font to use for the window.

#### `template`

Sets the template for the notification message. The syntax is based on [Jinja2](http://jinja.pocoo.org/) and [Django](https://docs.djangoproject.com/en/3.1/topics/templates/) templates.

Simply, there are 3 kinds of delimiters:

<!-- {% raw %} -->

- `{{` and `}}` for expressions
- `{%` or `{%-` and `%}` or `-%}` for statements
- `{#` and `#}` for comments

<!-- {% endraw %} -->

See [Tera documentation](https://tera.netlify.app/docs/#templates) for more information about [control structures](https://tera.netlify.app/docs/#control-structures), [built-in filters](https://tera.netlify.app/docs/#built-ins), etc.

##### Context

Context is the model that holds the required data for template rendering. The [JSON](https://en.wikipedia.org/wiki/JSON) format is used in the following example for the representation of a context.

```json
{
  "app_name": "runst",
  "summary": "example",
  "body": "this is a notification 🐻",
  "urgency": "normal",
  "unread_count": 1,
  "timestamp": 1672426610
}
```

##### Styling

[Pango](https://pango.gnome.org/) is used for text rendering. The markup documentation can be found [here](https://docs.gtk.org/Pango/pango_markup.html).

A few examples would be:

- `<b>bold text</b>`: **bold text**
- `<span foreground="blue">blue text</span>`: <span style="color:blue">blue text</span>
- `<tt>monospace text</tt>`: <tt>monospace text</tt>

### Urgency configuration

## Why this exists?

I have been a user of [dunst](https://github.com/dunst-project/dunst) for a long time. However, they made some [uncool breaking changes](https://github.com/dunst-project/dunst/issues/940) in [v1.7.0](https://github.com/dunst-project/dunst/releases/tag/v1.7.0) and it completely broke my configuration. That day, I refused to update `dunst` (I was too lazy to re-configure) and decided to write my own notification server using Rust.

I wanted to keep `runst` simple since the way I use `dunst` was really simple. I was only showing an overlay window on top of [i3status](https://github.com/i3/i3status) as shown below:

![runst use case](assets/runst-demo2.gif)

And that's how `runst` is born.

## Similar projects

- [wired-notify](https://github.com/Toqozz/wired-notify)

## License

Licensed under either of [Apache License Version 2.0](http://www.apache.org/licenses/LICENSE-2.0) or [The MIT License](http://opensource.org/licenses/MIT) at your option.

## Copyright

Copyright © 2022, [Orhun Parmaksız](mailto:orhunparmaksiz@gmail.com)
