use crate::{
    app::{App, HandyDandyRectBuilder},
    compositor::Compositor
};
use webrender::api::{*, units::*};
use euclid::Angle;
use std::path::PathBuf;

struct Animation {
    property_key0: PropertyBindingKey<LayoutTransform>,
    property_key1: PropertyBindingKey<LayoutTransform>,
    property_key2: PropertyBindingKey<LayoutTransform>,
    opacity_key: PropertyBindingKey<f32>,
    opacity: f32,
    angle0: f32,
    angle1: f32,
    angle2: f32,
}

impl Animation {
    fn add_rounded_rect(
        &mut self,
        bounds: LayoutRect,
        color: ColorF,
        builder: &mut DisplayListBuilder,
        pipeline_id: PipelineId,
        property_key: PropertyBindingKey<LayoutTransform>,
        opacity_key: Option<PropertyBindingKey<f32>>,
    ) {
        let filters = match opacity_key {
            Some(opacity_key) => {
                vec![
                    FilterOp::Opacity(PropertyBinding::Binding(opacity_key, self.opacity), self.opacity),
                ]
            }
            None => {
                vec![]
            }
        };

        let spatial_id = builder.push_reference_frame(
            bounds.origin,
            SpatialId::root_scroll_node(pipeline_id),
            TransformStyle::Flat,
            PropertyBinding::Binding(property_key, LayoutTransform::identity()),
            ReferenceFrameKind::Transform,
        );

        builder.push_simple_stacking_context_with_filters(
            LayoutPoint::zero(),
            spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
            &filters,
            &[],
            &[]
        );

        let space_and_clip = SpaceAndClipInfo {
            spatial_id,
            clip_id: ClipId::root(pipeline_id),
        };
        let clip_bounds = LayoutRect::new(LayoutPoint::zero(), bounds.size);
        let complex_clip = ComplexClipRegion {
            rect: clip_bounds,
            radii: BorderRadius::uniform(30.0),
            mode: ClipMode::Clip,
        };
        let clip_id = builder.define_clip_rounded_rect(
            &space_and_clip,
            complex_clip,
        );

        // Fill it with a white rect
        builder.push_rect(
            &CommonItemProperties::new(
                LayoutRect::new(LayoutPoint::zero(), bounds.size),
                SpaceAndClipInfo {
                    spatial_id,
                    clip_id,
                }
            ),
            LayoutRect::new(LayoutPoint::zero(), bounds.size),
            color,
        );

        builder.pop_stacking_context();
        builder.pop_reference_frame();
    }

    fn transform(
        &mut self,
        api: &mut RenderApi,
        document_id: DocumentId,
        (delta_angle, delta_opacity): (f32, f32)
    ) {
        // Update the transform based on the keyboard input and push it to
        // webrender using the generate_frame API. This will recomposite with
        // the updated transform.
        self.opacity += delta_opacity;
        self.angle0 += delta_angle * 0.1;
        self.angle1 += delta_angle * 0.2;
        self.angle2 -= delta_angle * 0.15;

        let xf0 = LayoutTransform::create_rotation(0.0, 0.0, 1.0, Angle::radians(self.angle0));
        let xf1 = LayoutTransform::create_rotation(0.0, 0.0, 1.0, Angle::radians(self.angle1));
        let xf2 = LayoutTransform::create_rotation(0.0, 0.0, 1.0, Angle::radians(self.angle2));

        let mut txn = Transaction::new();
        txn.update_dynamic_properties(
            DynamicProperties {
                transforms: vec![
                    PropertyValue {
                        key: self.property_key0,
                        value: xf0,
                    },
                    PropertyValue {
                        key: self.property_key1,
                        value: xf1,
                    },
                    PropertyValue {
                        key: self.property_key2,
                        value: xf2,
                    },
                ],
                floats: vec![
                    PropertyValue {
                        key: self.opacity_key,
                        value: self.opacity,
                    }
                ],
                colors: vec![],
            },
        );
        txn.generate_frame();
        api.send_transaction(document_id, txn);
    }
}

impl App for Animation {
    const TITLE: &'static str = "Aimation Example";
    const SIZE: (u32, u32) = (1200, 900);

    fn clear_color(&self) -> Option<ColorF> {
        Some(ColorF::new(1.0, 1.0, 1.0, 1.0))
    }

    fn add_font(&self) -> Option<(PathBuf, f32)> {
        None
    }

    fn build_display_list(
        &mut self,
        compositor: &mut Compositor,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        _font_instance_key: Option<FontInstanceKey>
    ) -> DisplayListBuilder {
        let mut builder = DisplayListBuilder::new(pipeline_id, compositor.get_layout_size());

        let opacity_key = self.opacity_key;

        let bounds = (150, 150).to(250, 250);
        let key0 = self.property_key0;
        self.add_rounded_rect(bounds, ColorF::new(1.0, 0.0, 0.0, 0.5), &mut builder, pipeline_id, key0, Some(opacity_key));

        let bounds = (600, 300).to(800, 500);
        let key1 = self.property_key1;
        self.add_rounded_rect(bounds, ColorF::new(0.0, 1.0, 0.0, 0.5), &mut builder, pipeline_id, key1, None);

        let bounds = (200, 500).to(350, 580);
        let key2 = self.property_key2;
        self.add_rounded_rect(bounds, ColorF::new(0.0, 0.0, 1.0, 0.5), &mut builder, pipeline_id, key2, None);

        builder
    }

    fn on_event(&mut self, event: winit::WindowEvent, api: &mut RenderApi, document_id: DocumentId) -> bool {
        let mut rebuild_display_list = false;

        match event {
            winit::WindowEvent::KeyboardInput {
                input: winit::KeyboardInput {
                    state: winit::ElementState::Released,
                    virtual_keycode: Some(key),
                    ..
                },
                ..
            } => {
                let (delta_angle, delta_opacity) = match key {
                    winit::VirtualKeyCode::Down => (0.0, -0.1),
                    winit::VirtualKeyCode::Up => (0.0, 0.1),
                    winit::VirtualKeyCode::Right => (1.0, 0.0),
                    winit::VirtualKeyCode::Left => (-1.0, 0.0),
                    winit::VirtualKeyCode::R => {
                        rebuild_display_list = true;
                        (0.0, 0.0)
                    }
                    _ => return false,
                };

                self.transform(api, document_id, (delta_angle, delta_opacity));
            }
            _ => ()
        }

        rebuild_display_list
    }
}

pub fn run() {
    let mut animation_app = Animation {
        property_key0: PropertyBindingKey::new(42), // arbitrary magic number
        property_key1: PropertyBindingKey::new(44), // arbitrary magic number
        property_key2: PropertyBindingKey::new(45), // arbitrary magic number
        opacity_key: PropertyBindingKey::new(43),
        opacity: 0.5,
        angle0: 0.0,
        angle1: 0.0,
        angle2: 0.0,
    };
    crate::app::run(&mut animation_app, None);
}
