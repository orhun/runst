use crate::error::Result;
use x11rb::connection::Connection;
use x11rb::protocol::{xproto::*, Event};
use x11rb::rust_connection::RustConnection;
use x11rb::COPY_DEPTH_FROM_PARENT;

/// Wrapper for X11 [`connection`] and [`screen`].
///
/// [`connection`]:  x11rb::rust_connection::RustConnection
/// [`screen`]: x11rb::protocol::xproto::Screen
pub struct X11 {
    connection: RustConnection,
    screen: Screen,
    window_id: u32,
}

impl X11 {
    /// Initializes the X11 connection.
    pub fn init(screen_num: Option<usize>) -> Result<Self> {
        let (connection, default_screen_num) = x11rb::connect(None)?;
        let setup_info = connection.setup();
        let screen = setup_info.roots[screen_num.unwrap_or(default_screen_num)].clone();
        Ok(Self {
            connection,
            screen,
            window_id: 0,
        })
    }

    /// Creates a window.
    pub fn create_window(&mut self) -> Result<()> {
        let window_id = self.connection.generate_id()?;
        self.connection.create_window(
            COPY_DEPTH_FROM_PARENT,
            window_id,
            self.screen.root,
            0,
            0,
            200,
            30,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .background_pixel(self.screen.white_pixel)
                .override_redirect(1)
                .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),
        )?;
        self.window_id = window_id;
        Ok(())
    }

    /// Shows the window.
    pub fn show_window(&self) -> Result<()> {
        self.connection.map_window(self.window_id)?;
        self.connection.flush()?;
        Ok(())
    }

    /// Hides the window.
    pub fn hide_window(&self) -> Result<()> {
        self.connection.unmap_window(self.window_id)?;
        self.connection.flush()?;
        Ok(())
    }

    /// Handles the events.
    pub fn handle_events(&self) -> Result<()> {
        loop {
            let event = self.connection.wait_for_event()?;
            println!("{:?})", event);
            match event {
                Event::ButtonPress(_) => {
                    self.hide_window()?;
                }
                _ => {}
            }
        }
    }
}
