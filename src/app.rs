use gleam::gl;
use surfman::GLApi;
use webrender::{Renderer, RendererOptions, ShaderPrecacheFlags};
use webrender::api::{
    RenderApi, DisplayListBuilder, FontInstanceKey,
    RenderNotifier, DocumentId, PipelineId, DebugCommand, DebugFlags,
    ExternalImageHandler, OutputImageHandler, ColorF, Epoch,
    units::{LayoutRect, LayoutPoint, LayoutSize}
};
use winit::{
    EventsLoop, EventsLoopProxy,
    VirtualKeyCode, Event, WindowEvent, ControlFlow,
    dpi::LogicalSize
};
use std::{cell::RefCell, rc::Rc, path::PathBuf};
use crate::{
    window::Window,
    compositor::Compositor
};

struct Notifier {
    events_proxy: EventsLoopProxy,
}

impl Notifier {
    fn new(events_proxy: EventsLoopProxy) -> Notifier {
        Notifier { events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        #[cfg(not(target_os = "android"))]
        let _ = self.events_proxy.wakeup();
    }

    fn new_frame_ready(&self,
                       _: DocumentId,
                       _scrolled: bool,
                       _composite_needed: bool,
                       _render_time: Option<u64>) {
        self.wake_up();
    }
}

pub trait HandyDandyRectBuilder {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect;
    fn by(&self, w: i32, h: i32) -> LayoutRect;
}
// Allows doing `(x, y).to(x2, y2)` or `(x, y).by(width, height)` with i32
// values to build a f32 LayoutRect
impl HandyDandyRectBuilder for (i32, i32) {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect {
        LayoutRect::new(
            LayoutPoint::new(self.0 as f32, self.1 as f32),
            LayoutSize::new((x2 - self.0) as f32, (y2 - self.1) as f32),
        )
    }

    fn by(&self, w: i32, h: i32) -> LayoutRect {
        LayoutRect::new(
            LayoutPoint::new(self.0 as f32, self.1 as f32),
            LayoutSize::new(w as f32, h as f32),
        )
    }
}

pub trait App {
	const PRECACHE_SHADER_FLAGS: ShaderPrecacheFlags = ShaderPrecacheFlags::EMPTY;
	const SIZE: (u32, u32) = (800, 600);
    const TITLE: &'static str = "Example";

    fn clear_color(&self) -> Option<ColorF> {
        Some(ColorF::new(0.3, 0.0, 0.0, 1.0))
    }

    fn add_font(&self) -> Option<(PathBuf, f32)> {
        None
    }

    fn build_display_list(
        &mut self,
        compositor: &mut Compositor,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        font_instance_key: Option<FontInstanceKey>
    ) -> DisplayListBuilder;

    fn on_event(
        &mut self,
        _: winit::WindowEvent,
        _: &mut RenderApi,
        _: DocumentId,
    ) -> bool {
        false
    }

    fn get_image_handlers(
        &mut self,
        _gl: &dyn gl::Gl,
    ) -> (Option<Box<dyn ExternalImageHandler>>,
          Option<Box<dyn OutputImageHandler>>) {
        (None, None)
    }
    fn draw_custom(&mut self, _gl: &dyn gl::Gl) {}
}

pub fn run<E: App>(
    app: &mut E,
    options: Option<RendererOptions>,
) {
    env_logger::init();

    #[cfg(target_os = "macos")]
    {
        use core_foundation::{self as cf, base::TCFType};
        let i = cf::bundle::CFBundle::main_bundle().info_dictionary();
        let mut i = unsafe { i.to_mutable() };
        i.set(
            cf::string::CFString::new("NSSupportsAutomaticGraphicsSwitching"),
            cf::boolean::CFBoolean::true_value().into_CFType(),
        );
    }

    let events_loop = Rc::new(RefCell::new(EventsLoop::new()));
    let win = Window::new(
        E::TITLE,
        LogicalSize::new(E::SIZE.0 as f64, E::SIZE.1 as f64),
        events_loop.clone()
    );

    // Initialize surfman
    let webrender_surfman = win.webrender_surfman();

    // Get GL bindings
    let webrender_gl = match webrender_surfman.connection().gl_api() {
        GLApi::GL => unsafe { gl::GlFns::load_with(|s| webrender_surfman.get_proc_address(s)) },
        GLApi::GLES => unsafe {
            gl::GlesFns::load_with(|s| webrender_surfman.get_proc_address(s))
        },
    };

    // Make sure the gl context is made current.
    webrender_surfman.make_gl_context_current().unwrap();

    println!("OpenGL version {}", webrender_gl.get_string(gl::VERSION));

    let coordinates = win.get_coordinates();
    let device_pixel_ratio = coordinates.hidpi_factor.get();

    println!("Device pixel ratio: {}", device_pixel_ratio);

    let notifier = Box::new(Notifier::new(events_loop.borrow().create_proxy()));

    let (mut webrender, sender) = Renderer::new(
        webrender_gl.clone(),
        notifier,
        RendererOptions {
            device_pixel_ratio,
            clear_color: app.clear_color(),
            ..options.unwrap_or_default()
        },
        None,
        coordinates.framebuffer
    )
    .expect("Unable to initialize webrender!");

    let webrender_api = sender.create_api();

    // webrender_api.send_debug_cmd(DebugCommand::SetFlags(DebugFlags::PROFILER_DBG));

    let document_id = webrender_api.add_document(coordinates.framebuffer, 0);

    // set image handler
    let (external, output) = app.get_image_handlers(&*webrender_gl);
    if let Some(output_image_handler) = output {
        webrender.set_output_image_handler(output_image_handler);
    }
    if let Some(external_image_handler) = external {
        webrender.set_external_image_handler(external_image_handler);
    }

    let epoch = Epoch(0);
    let pipeline_id = PipelineId(0, 0);

    let mut compositor = Compositor::new(Rc::new(win), webrender, document_id, webrender_api, webrender_surfman, webrender_gl.clone());

    let font_instance_key =  app.add_font().map(|font| compositor.set_font_instance(font, document_id));

    let builder = app.build_display_list(
        &mut compositor,
        pipeline_id,
        document_id,
        font_instance_key
    );
    compositor.send_display_list(epoch, pipeline_id, builder);


    println!("Entering event loop");

    // run event_loop
    events_loop.borrow_mut().run_forever(|global_event| {
        let mut custom_event = true;
        let win_event = match global_event {
            Event::WindowEvent { event, .. } => event,
            _ => return ControlFlow::Continue,
        };

        match win_event {
            WindowEvent::CloseRequested => return ControlFlow::Break,
            | winit::WindowEvent::AxisMotion { .. }
            | winit::WindowEvent::CursorMoved { .. } => {
                custom_event = app.on_event(
                        win_event,
                        compositor.get_webrender_api(),
                        document_id,
                    );
                // skip high-frequency events from triggering a frame draw.
                if !custom_event {
                    return winit::ControlFlow::Continue;
                }
            },
            WindowEvent::KeyboardInput {
                input: winit::KeyboardInput {
                    state: winit::ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                },
                ..
            } => match key {
                VirtualKeyCode::Escape => return ControlFlow::Break,
                _ => {},
            },
            other => custom_event = app.on_event(
                other,
                compositor.get_webrender_api(),
                document_id
            )
        }

        if custom_event {
            let builder = app.build_display_list(
                &mut compositor,
                pipeline_id,
                document_id,
                font_instance_key
            );
            compositor.send_display_list(epoch, pipeline_id, builder);
        }

        compositor.composite();
        app.draw_custom(&*webrender_gl.clone());
        compositor.present();

        ControlFlow::Continue
    });

    compositor.deinit();
}
