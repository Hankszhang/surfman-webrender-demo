use crate::{
    app::App,
    compositor::Compositor
};
use webrender::api::{
    SpaceAndClipInfo, PrimitiveFlags, CommonItemProperties, DisplayListBuilder,
    PipelineId, ColorF, GlyphInstance, FontInstanceKey, DocumentId,
	units::{LayoutRect, LayoutPoint, LayoutSize}
};
use std::{path::PathBuf, env::current_dir};

struct Basic {}

impl App for Basic {
    const TITLE: &'static str = "Basic Example";

    fn clear_color(&self) -> Option<ColorF> {
        Some(ColorF::new(0.3, 0.0, 0.0, 1.0))
    }

    fn add_font(&self) -> Option<(PathBuf, f32)> {
        Some((current_dir().unwrap().join("res/fonts/FreeSans.ttf"), 32.0))
    }

    fn build_display_list(
        &mut self,
        compositor: &mut Compositor,
        pipeline_id: PipelineId,
        _document_id: DocumentId,
        font_instance_key: Option<FontInstanceKey>
    ) -> DisplayListBuilder {
        let mut builder = DisplayListBuilder::new(pipeline_id, compositor.get_layout_size());

        let space_and_clip = SpaceAndClipInfo::root_scroll(pipeline_id);

        let bounds = LayoutRect::new(LayoutPoint::zero(), builder.content_size());
        builder.push_simple_stacking_context(
            bounds.origin,
            space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        builder.push_rect(
            &CommonItemProperties::new(
                LayoutRect::new(
                    LayoutPoint::new(100.0, 200.0),
                    LayoutSize::new(100.0, 200.0),
                ),
                space_and_clip,
            ),
            LayoutRect::new(
                LayoutPoint::new(100.0, 200.0),
                LayoutSize::new(100.0, 200.0),
            ),
            ColorF::new(0.0, 1.0, 0.0, 1.0)
        );

        let text_bounds = LayoutRect::new(
            LayoutPoint::new(100.0, 50.0),
            LayoutSize::new(700.0, 200.0)
        );

        let glyphs = vec![
            GlyphInstance {
                index: 48,
                point: LayoutPoint::new(100.0, 100.0),
            },
            GlyphInstance {
                index: 68,
                point: LayoutPoint::new(150.0, 100.0),
            },
            GlyphInstance {
                index: 80,
                point: LayoutPoint::new(200.0, 100.0),
            },
            GlyphInstance {
                index: 82,
                point: LayoutPoint::new(250.0, 100.0),
            },
            GlyphInstance {
                index: 81,
                point: LayoutPoint::new(300.0, 100.0),
            },
            GlyphInstance {
                index: 3,
                point: LayoutPoint::new(350.0, 100.0),
            },
            GlyphInstance {
                index: 86,
                point: LayoutPoint::new(400.0, 100.0),
            },
            GlyphInstance {
                index: 79,
                point: LayoutPoint::new(450.0, 100.0),
            },
            GlyphInstance {
                index: 72,
                point: LayoutPoint::new(500.0, 100.0),
            },
            GlyphInstance {
                index: 83,
                point: LayoutPoint::new(550.0, 100.0),
            },
            GlyphInstance {
                index: 87,
                point: LayoutPoint::new(600.0, 100.0),
            },
            GlyphInstance {
                index: 17,
                point: LayoutPoint::new(650.0, 100.0),
            },
        ];

        builder.push_text(
            &CommonItemProperties::new(
                text_bounds,
                space_and_clip,
            ),
            text_bounds,
            &glyphs,
            font_instance_key.unwrap(),
            ColorF::new(1.0, 1.0, 0.0, 1.0),
            None,
        );


        builder.pop_stacking_context();

        builder
    }
}

pub fn run() {
    let mut basic_app = Basic {};
    crate::app::run(&mut basic_app, None);
}
