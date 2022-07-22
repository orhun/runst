use crate::config::{Config, GlobalConfig};
use crate::dbus::{Notification, NotificationContext};
use crate::error::{Error, Result};
use cairo::{
    Context as CairoContext, XCBConnection as CairoXCBConnection, XCBDrawable, XCBSurface,
    XCBVisualType,
};
use colorsys::ColorAlpha;
use pango::{Context as PangoContext, FontDescription, Layout as PangoLayout};
use pangocairo::functions as pango_functions;
use std::sync::{Arc, RwLock};
use tinytemplate::TinyTemplate;
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
    pub fn create_window(&mut self, config: &GlobalConfig) -> Result<X11Window> {
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
            config.geometry.x.try_into()?,
            config.geometry.y.try_into()?,
            config.geometry.width.try_into()?,
            config.geometry.height.try_into()?,
            0,
            WindowClass::INPUT_OUTPUT,
            visual_id,
            &CreateWindowAux::new()
                .border_pixel(self.screen.white_pixel)
                .override_redirect(1)
                .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),
        )?;
        let surface = XCBSurface::create(
            &self.cairo,
            &XCBDrawable(window_id),
            &visual,
            config.geometry.width.try_into()?,
            config.geometry.height.try_into()?,
        )?;
        let context = CairoContext::new(&surface)?;
        X11Window::new(
            window_id,
            context,
            &config.font,
            Box::leak(config.format.to_string().into_boxed_str()),
        )
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
    pub fn handle_events<F>(
        &self,
        window: Arc<X11Window>,
        notifications: Arc<RwLock<Vec<Notification>>>,
        config: Arc<Config>,
        on_press: F,
    ) -> Result<()>
    where
        F: Fn(&Notification),
    {
        loop {
            self.connection.flush()?;
            let event = self.connection.wait_for_event()?;
            let mut event_opt = Some(event);
            while let Some(event) = event_opt {
                println!("{:?}", event);
                match event {
                    Event::Expose(_) => {
                        window.draw(&notifications, &config)?;
                    }
                    Event::ButtonPress(_) => {
                        let mut notifications = notifications
                            .write()
                            .expect("failed to retrieve notifications");
                        let mut unread_notifications = notifications
                            .iter_mut()
                            .filter(|v| !v.is_read)
                            .collect::<Vec<&mut Notification>>();
                        unread_notifications[0].is_read = true;
                        on_press(unread_notifications[0]);
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
    /// Text format.
    pub template: TinyTemplate<'static>,
}

unsafe impl Send for X11Window {}
unsafe impl Sync for X11Window {}

impl X11Window {
    /// Creates a new instance of window.
    pub fn new(
        id: u32,
        cairo_context: CairoContext,
        font: &str,
        format: &'static str,
    ) -> Result<Self> {
        let pango_context = pango_functions::create_context(&cairo_context)
            .ok_or_else(|| Error::PangoOther(String::from("failed to create context")))?;
        let layout = PangoLayout::new(&pango_context);
        let font_description = FontDescription::from_string(font);
        pango_context.set_font_description(&font_description);
        let mut template = TinyTemplate::new();
        template.add_template("notification_message", format)?;
        Ok(Self {
            id,
            cairo_context,
            pango_context,
            layout,
            template,
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
    fn draw(&self, notifications: &Arc<RwLock<Vec<Notification>>>, config: &Config) -> Result<()> {
        let notifications = notifications
            .read()
            .expect("failed to retrieve notifications");
        if let Some(notification) = &notifications.iter().rev().filter(|v| !v.is_read).last() {
            let urgency_config = config.get_urgency_config(&notification.urgency);
            let message = self.template.render(
                "notification_message",
                &NotificationContext {
                    app_name: &notification.app_name,
                    summary: &notification.summary,
                    body: &notification.body,
                    urgency: &urgency_config.text,
                },
            )?;
            let background_color = urgency_config.background;
            self.cairo_context.set_source_rgba(
                background_color.red() / 255.0,
                background_color.green() / 255.0,
                background_color.blue() / 255.0,
                background_color.alpha(),
            );
            self.cairo_context.fill()?;
            self.cairo_context.paint()?;
            let foreground_color = urgency_config.foreground;
            self.cairo_context.set_source_rgba(
                foreground_color.red() / 255.0,
                foreground_color.green() / 255.0,
                foreground_color.blue() / 255.0,
                foreground_color.alpha(),
            );
            self.cairo_context.move_to(0., 0.);
            self.layout.set_markup(&message);
            pango_functions::show_layout(&self.cairo_context, &self.layout);
        }
        Ok(())
    }
}
