use crate::config::Geometry;
use crate::dbus::Notification;
use crate::error::{Error, Result};
use cairo::{
    Context as CairoContext, XCBConnection as CairoXCBConnection, XCBDrawable, XCBSurface,
    XCBVisualType,
};
use pango::{Context as PangoContext, FontDescription, Layout as PangoLayout};
use pangocairo::functions as pango_functions;
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
    pub fn create_window(&mut self, geometry: Geometry, font: String) -> Result<X11Window> {
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
            geometry.x.try_into()?,
            geometry.y.try_into()?,
            geometry.width.try_into()?,
            geometry.height.try_into()?,
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
        X11Window::new(window_id, context, font)
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

    /// Hides the given X11 window.
    pub fn hide_window(&self, window: &X11Window) -> Result<()> {
        window.hide(&self.connection)?;
        self.connection.flush()?;
        Ok(())
    }

    /// Handles the events.
    pub fn handle_events<F>(&self, window: Arc<RwLock<X11Window>>, on_press: F) -> Result<()>
    where
        F: Fn(Option<&Notification>),
    {
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
                        on_press(window.content.as_ref())
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
    /// Window ID.
    pub id: u32,
    /// Graphics renderer context.
    pub cairo_context: CairoContext,
    /// Text renderer context.
    pub pango_context: PangoContext,
    /// Window layout.
    pub layout: PangoLayout,
    /// Content of the window.
    pub content: Option<Notification>,
}

unsafe impl Send for X11Window {}
unsafe impl Sync for X11Window {}

impl X11Window {
    /// Creates a new instance of window.
    pub fn new(id: u32, cairo_context: CairoContext, font: String) -> Result<Self> {
        let pango_context = pango_functions::create_context(&cairo_context)
            .ok_or_else(|| Error::PangoOther(String::from("failed to create context")))?;
        let layout = PangoLayout::new(&pango_context);
        let font_description = FontDescription::from_string(&font);
        pango_context.set_font_description(&font_description);
        Ok(Self {
            id,
            cairo_context,
            pango_context,
            layout,
            content: None,
        })
    }

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
            self.layout.set_text(&content.to_string());
            self.cairo_context.set_source_rgb(1., 1., 1.);
            self.cairo_context.move_to(0., 0.);
            pango_functions::show_layout(&self.cairo_context, &self.layout);
        }
        Ok(())
    }
}
