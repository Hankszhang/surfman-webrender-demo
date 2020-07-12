use crate::webrender_surfman::WebrenderSurfman;
use euclid::{Point2D, Scale, Size2D};
use std::{cell::{Cell, RefCell}, rc::Rc};
use surfman::{Connection, SurfaceType};
use webrender::api::{
    units::{DeviceIntPoint, DeviceIntRect, DeviceIntSize, DevicePixel, LayoutSize},
    *,
};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
    EventsLoop, WindowBuilder,
};

#[derive(Clone, Copy, Debug)]
pub enum DeviceIndependentPixel {}

#[derive(Clone, Copy, Debug)]
pub struct EmbedderCoordinates {
    /// The pixel density of the display.
    pub hidpi_factor: Scale<f32, DeviceIndependentPixel, DevicePixel>,
    /// Size of the screen.
    pub screen: DeviceIntSize,
    /// Size of the native window.
    pub window: (DeviceIntSize, DeviceIntPoint),
    /// Size of the GL buffer in the window.
    pub framebuffer: DeviceIntSize,
    /// Coordinates of the document within the framebuffer.
    pub viewport: DeviceIntRect,
    /// Size of layout
    pub layout: LayoutSize
}

impl EmbedderCoordinates {
    pub fn get_flipped_viewport(&self) -> DeviceIntRect {
        let fb_height = self.framebuffer.height;
        let mut view = self.viewport.clone();
        view.origin.y = fb_height - view.origin.y - view.size.height;
        DeviceIntRect::from_untyped(&view.to_untyped())
    }
}

pub struct Window {
    winit_window: winit::Window,
    webrender_surfman: WebrenderSurfman,
    screen_size: Size2D<u32, DeviceIndependentPixel>,
    pub inner_size: Cell<Size2D<u32, DeviceIndependentPixel>>,
}

impl Window {
    pub fn new(name: &'static str, size: LogicalSize, events_loop: Rc<RefCell<EventsLoop>>) -> Self {
        let window_builder = WindowBuilder::new()
            .with_title(name)
            // .with_decorations(true)
            .with_resizable(false)
            .with_visibility(true)
            .with_dimensions(size)
            .with_multitouch();

        let winit_window = window_builder
            .build(&events_loop.borrow())
            .expect("Faild to create window");

        let primary_monitor = events_loop.borrow().get_primary_monitor();

        let PhysicalSize {
            width: screen_width,
            height: screen_height,
        } = primary_monitor.get_dimensions();

        let screen_size = Size2D::new(screen_width as u32, screen_height as u32);

        let LogicalSize { width, height } = winit_window
            .get_inner_size()
            .expect("Failed to get window inner size.");
        let inner_size = Size2D::new(width as u32, height as u32);

        winit_window.show();

        // initialize surfman
        let connection =
            Connection::from_winit_window(&winit_window).expect("Faild to create connection");
        let adapter = connection
            .create_adapter()
            .expect("Failed to create adapter");
        let native_widget = connection
            .create_native_widget_from_winit_window(&winit_window)
            .expect("Failed to create native widget");
        let surface_type = SurfaceType::Widget { native_widget };
        let webrender_surfman = WebrenderSurfman::create(&connection, &adapter, surface_type)
            .expect("Failed to create webrender surfman");

        println!("Created window {:?}", winit_window.id());

        Window {
            winit_window,
            webrender_surfman,
            screen_size,
            inner_size: Cell::new(inner_size),
        }
    }

    fn device_hidpi_factor(&self) -> Scale<f32, DeviceIndependentPixel, DevicePixel> {
        Scale::new(self.winit_window.get_hidpi_factor() as f32)
    }

    pub fn webrender_surfman(&self) -> WebrenderSurfman {
        self.webrender_surfman.clone()
    }

    pub fn get_coordinates(&self) -> EmbedderCoordinates {
        let dpr = self.device_hidpi_factor();
        let LogicalSize { width, height } = self
            .winit_window
            .get_outer_size()
            .expect("Failed to get window outer size.");
        let LogicalPosition { x, y } = self
            .winit_window
            .get_position()
            .unwrap_or(LogicalPosition::new(0., 0.));
        let win_size = (Size2D::new(width as f32, height as f32) * dpr).to_i32();
        let win_origin = (Point2D::new(x as f32, y as f32) * dpr).to_i32();
        let screen = (self.screen_size.to_f32() * dpr).to_i32();

        let LogicalSize { width, height } = self
            .winit_window
            .get_inner_size()
            .expect("Failed to get window inner size.");
        let inner_size = (Size2D::new(width as f32, height as f32) * dpr).to_i32();
        let viewport = DeviceIntRect::new(Point2D::zero(), inner_size);
        let framebuffer = DeviceIntSize::from_untyped(viewport.size.to_untyped());
        let hidpi_factor = self.device_hidpi_factor();
        let layout = framebuffer.to_f32() / Scale::new(hidpi_factor.get());
        EmbedderCoordinates {
            viewport,
            framebuffer,
            window: (win_size, win_origin),
            screen,
            hidpi_factor,
            layout
        }
    }
}
