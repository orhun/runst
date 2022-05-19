use crate::dbus::Notification;
use crate::error::{Error, Result};
use cairo::{
    Context as CairoContext, XCBConnection as CairoXCBConnection, XCBDrawable, XCBSurface,
    XCBVisualType,
};
use std::sync::{Arc, RwLock};
use x11rb::connection::Connection;
use x11rb::protocol::{xproto::*, Event};
use x11rb::xcb_ffi::XCBConnection;
use x11rb::COPY_DEPTH_FROM_PARENT;

/// Rust version of XCB's [`xcb_visualtype_t`] struct.
///
/// [`xcb_visualtype_t`]: https://xcb.freedesktop.org/manual/structxcb__visualtype__t.html
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct xcb_visualtype_t {
    visual_id: u32,
    class: u8,
    bits_per_rgb_value: u8,
    colormap_entries: u16,
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
    pad0: [u8; 4],
}

impl From<Visualtype> for xcb_visualtype_t {
    fn from(value: Visualtype) -> xcb_visualtype_t {
        xcb_visualtype_t {
            visual_id: value.visual_id,
            class: value.class.into(),
            bits_per_rgb_value: value.bits_per_rgb_value,
            colormap_entries: value.colormap_entries,
            red_mask: value.red_mask,
            green_mask: value.green_mask,
            blue_mask: value.blue_mask,
            pad0: [0; 4],
        }
    }
}

/// Wrapper for X11 [`connection`] and [`screen`].
///
/// [`connection`]: XCBConnection
/// [`screen`]: x11rb::protocol::xproto::Screen
pub struct X11 {
    connection: XCBConnection,
    cairo: CairoXCBConnection,
    screen: Screen,
}

unsafe impl Send for X11 {}
unsafe impl Sync for X11 {}

impl X11 {
    /// Initializes the X11 connection.
    pub fn init(screen_num: Option<usize>) -> Result<Self> {
        let (connection, default_screen_num) = XCBConnection::connect(None)?;
        let setup_info = connection.setup();
        let screen = setup_info.roots[screen_num.unwrap_or(default_screen_num)].clone();
        let cairo =
            unsafe { CairoXCBConnection::from_raw_none(connection.get_raw_xcb_connection() as _) };
        Ok(Self {
            connection,
            screen,
            cairo,
        })
    }

    /// Creates a window.
    pub fn create_window(&mut self) -> Result<X11Window> {
        let visual_id = self.screen.root_visual;
        let mut visual_type = self
            .find_xcb_visualtype(visual_id)
            .ok_or_else(|| Error::X11Other(String::from("cannot find a XCB visual type")))?;
        let visual = unsafe { XCBVisualType::from_raw_none(&mut visual_type as *mut _ as _) };
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
            visual_id,
            &CreateWindowAux::new()
                .border_pixel(self.screen.white_pixel)
                .override_redirect(1)
                .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),
        )?;
        let surface = XCBSurface::create(&self.cairo, &XCBDrawable(window_id), &visual, 200, 30)?;
        let context = CairoContext::new(&surface)?;
        Ok(X11Window {
            id: window_id,
            surface,
            context,
            content: None,
        })
    }

    /// Find a `xcb_visualtype_t` based on its ID number
    fn find_xcb_visualtype(&self, visual_id: u32) -> Option<xcb_visualtype_t> {
        for root in &self.connection.setup().roots {
            for depth in &root.allowed_depths {
                for visual in &depth.visuals {
                    if visual.visual_id == visual_id {
                        return Some((*visual).into());
                    }
                }
            }
        }
        None
    }

    /// Shows the given X11 window.
    pub fn show_window(&self, window: &X11Window) -> Result<()> {
        window.show(&self.connection)?;
        self.connection.flush()?;
        Ok(())
    }

    /// Handles the events.
    pub fn handle_events(&self, window: Arc<RwLock<X11Window>>) -> Result<()> {
        println!("Handling events");
        loop {
            self.connection.flush()?;
            let event = self.connection.wait_for_event()?;
            let mut event_opt = Some(event);
            while let Some(event) = event_opt {
                println!("{:?}", event);
                let window = window.read().expect("failed to retrieve window");
                match event {
                    Event::Expose(_) => {
                        window.draw()?;
                    }
                    Event::ButtonPress(_) => {
                        window.hide(&self.connection)?;
                    }
                    _ => {}
                }
                event_opt = self.connection.poll_for_event()?;
            }
        }
    }
}

/// Represenation of a X11 window.
pub struct X11Window {
    id: u32,
    surface: XCBSurface,
    context: CairoContext,
    /// Content of the window.
    pub content: Option<Notification>,
}

unsafe impl Send for X11Window {}
unsafe impl Sync for X11Window {}

impl X11Window {
    /// Shows the window.
    fn show(&self, connection: &impl Connection) -> Result<()> {
        connection.map_window(self.id)?;
        Ok(())
    }

    /// Hides the window.
    fn hide(&self, connection: &impl Connection) -> Result<()> {
        connection.unmap_window(self.id)?;
        Ok(())
    }

    /// Draws the window content.
    fn draw(&self) -> Result<()> {
        if let Some(content) = &self.content {
            self.context.set_source_rgb(0.0, 0.0, 0.0);
            self.context.paint()?;
            self.context.set_source_rgb(1., 1., 1.);
            self.context.move_to(10.0, 30.0);
            self.context.set_font_size(20.0);
            self.context.show_text(&content.to_string())?;
            self.surface.flush();
        }
        Ok(())
    }
}
