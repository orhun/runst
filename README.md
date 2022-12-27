<div align="center">

  <a href="https://github.com/orhun/runst">
    <img src="assets/runst-logo.jpg" width="300">
  </a>

#### **`runst`** — A dead simple notification daemon 🔔💬

</div>

[Desktop notifications](https://wiki.archlinux.org/title/Desktop_notifications) are small, passive popup dialogs that notify the user of particular events in an asynchronous manner. These passive popups can automatically disappear after a short period of time.

`runst` is the server implementation of [freedesktop.org](https://www.freedesktop.org/wiki) - [Desktop Notifications Specification](https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html) and it can be used to receive notifications from applications via [D-Bus](https://www.freedesktop.org/wiki/Software/dbus/). As of now, only [X11](https://en.wikipedia.org/wiki/X_Window_System) is supported.

## Features

- Fully customizable notification window (size, location, text, colors).
- Template-powered ([Jinja2](http://jinja.pocoo.org/)/[Django](https://docs.djangoproject.com/en/3.1/topics/templates/)) notification text.
- Auto-clear notifications based on a fixed time or estimated read time.
- Run custom OS commands based on the matched notifications.

## Installation

## Usage

## Why this exists?

I have been an user of [dunst](https://github.com/dunst-project/dunst) for a long time. However, in my opinion, they made some [uncool breaking changes](https://github.com/dunst-project/dunst/issues/940) in [v1.7.0](https://github.com/dunst-project/dunst/releases/tag/v1.7.0).

## Similar projects

- [wired-notify](https://github.com/Toqozz/wired-notify)

## License

Licensed under either of [Apache License Version 2.0](http://www.apache.org/licenses/LICENSE-2.0) or [The MIT License](http://opensource.org/licenses/MIT) at your option.

## Copyright

Copyright © 2022, [Orhun Parmaksız](mailto:orhunparmaksiz@gmail.com)
