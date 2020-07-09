use gleam::gl;
use webrender::Renderer;
use webrender::api::{
    RenderApi, Transaction, FontInstanceKey,
    DocumentId, PipelineId, DisplayListBuilder, Epoch,
	units::LayoutSize
};
use euclid::Scale;
use crate::{
    webrender_surfman::WebrenderSurfman,
    window::Window
};
use std::{rc::Rc, path::PathBuf, fs::File, io::Read};

pub struct Compositor {
    window: Rc<Window>,
    /// The webrender renderer.
    webrender: Renderer,
    /// The webrender interface, if enabled.
    webrender_api: RenderApi,
    /// The surfman instance that webrender targets
    webrender_surfman: WebrenderSurfman,
    /// The GL bindings for webrender
    webrender_gl: Rc<dyn gleam::gl::Gl>,
    /// The active webrender document.
    document_id: DocumentId
}

impl Compositor {
    pub fn new(
        window: Rc<Window>,
        webrender: Renderer,
        document_id: DocumentId,
        webrender_api: RenderApi,
        webrender_surfman: WebrenderSurfman,
        webrender_gl: Rc<dyn gleam::gl::Gl>,
    ) -> Self {
        Self {
            window,
            webrender,
            document_id,
            webrender_api,
            webrender_surfman,
            webrender_gl
        }
    }

    pub fn deinit(self) {
        if let Err(err) = self.webrender_surfman.make_gl_context_current() {
            println!("Failed to make GL context current: {:?}", err);
        }

        self.webrender.deinit();
    }

    pub fn set_font_instance(&mut self, (font_path, glyph_size): (PathBuf, f32), document_id: DocumentId) -> FontInstanceKey {
        let font_key = self.webrender_api.generate_font_key();
        println!("load font path: {:?}", font_path);
        let font_bytes = load_file(font_path);

        let mut txn = Transaction::new();
        txn.add_raw_font(font_key, font_bytes, 0);

        let font_instance_key = self.webrender_api.generate_font_instance_key();
        txn.add_font_instance(font_instance_key, font_key, glyph_size, None, None, Vec::new());

        self.webrender_api.send_transaction(document_id, txn);

        println!("set font instance success, font_innstance_key={:?}", font_instance_key);

        font_instance_key
    }

    pub fn get_webrender_api(&mut self) -> &mut RenderApi {
        &mut self.webrender_api
    }

    pub fn send_display_list(
        &mut self,
        epoch: Epoch, 
        pipeline_id: PipelineId, 
        builder: DisplayListBuilder
    ) {
        let mut txn = Transaction::new();
        txn.set_display_list(
            epoch,
            None,
            self.get_layout_size(),
            builder.finalize(),
            true,
        );
        txn.set_root_pipeline(pipeline_id);
        txn.generate_frame();
        self.webrender_api.send_transaction(self.document_id, txn);
    }

    pub fn composite(&mut self) {
        if let Err(err) = self.webrender_surfman.make_gl_context_current() {
            println!("Failed to make GL context current: {:?}", err);
        }
        self.assert_no_gl_error();

        // Bind the webrender framebuffer
        let framebuffer_object = self
            .webrender_surfman
            .context_surface_info()
            .unwrap_or(None)
            .map(|info| info.framebuffer_object)
            .unwrap_or(0);
        self.webrender_gl
            .bind_framebuffer(gleam::gl::FRAMEBUFFER, framebuffer_object);
        self.assert_gl_framebuffer_complete();

        self.webrender.update();

        let size = self.window.get_coordinates().framebuffer;
        self.clear_background();
        self.webrender.render(size).ok();
    }

    pub fn present(&mut self) {
        // Perform the page flip. This will likely block for a while.
        if let Err(err) = self.webrender_surfman.present() {
            println!("Failed to present surface: {:?}", err);
        }
    }

    fn get_layout_size(&self) -> LayoutSize {
        let coordinates = self.window.get_coordinates();
        coordinates.viewport.size.to_f32() / Scale::new(coordinates.hidpi_factor.get())
    }

    fn assert_no_gl_error(&self) {
        debug_assert_eq!(self.webrender_gl.get_error(), gl::NO_ERROR);
    }

    fn assert_gl_framebuffer_complete(&self) {
        debug_assert_eq!(
            (
                self.webrender_gl.get_error(),
                self.webrender_gl
                    .check_frame_buffer_status(gl::FRAMEBUFFER)
            ),
            (gl::NO_ERROR, gl::FRAMEBUFFER_COMPLETE)
        );
    }

    fn clear_background(&self) {
        let gl = &self.webrender_gl;
        self.assert_gl_framebuffer_complete();

        // Make framebuffer fully transparent.
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(gl::COLOR_BUFFER_BIT);
        self.assert_gl_framebuffer_complete();

        // Make the viewport white.
        let viewport = self.window.get_coordinates().get_flipped_viewport();
        gl.scissor(
            viewport.origin.x,
            viewport.origin.y,
            viewport.size.width,
            viewport.size.height,
        );
        gl.clear_color(1.0, 1.0, 1.0, 1.0);
        gl.enable(gl::SCISSOR_TEST);
        gl.clear(gl::COLOR_BUFFER_BIT);
        gl.disable(gl::SCISSOR_TEST);
        self.assert_gl_framebuffer_complete();
    }
}

fn load_file(name: PathBuf) -> Vec<u8> {
    let mut file = File::open(name).unwrap();
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();
    buffer
}