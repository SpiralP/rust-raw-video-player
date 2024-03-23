use gstreamer::{
    glib::{self, subclass::types::ObjectSubclassIsExt},
    Element,
};
use gstreamer_app::AppSink;
use gstreamer_play::PlayVideoRenderer;

mod imp {
    use gstreamer::{
        glib::{self, object::Cast},
        prelude::{ElementExt, GstBinExtManual},
        Bin, Element, ElementFactory, GhostPad,
    };
    use gstreamer_app::AppSink;
    use gstreamer_play::subclass::prelude::*;
    use gstreamer_video::{VideoCapsBuilder, VideoFormat};
    use tracing::debug;

    #[derive(Debug)]
    pub struct Renderer {
        pub(super) videorate: Element,
        pub(super) capsfilter: Element,
        pub(super) appsink: AppSink,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Renderer {
        type Interfaces = (gstreamer_play::PlayVideoRenderer,);
        type ParentType = glib::Object;
        type Type = super::Renderer;

        const NAME: &'static str = "Renderer";

        fn with_class(_klass: &Self::Class) -> Self {
            let videorate = ElementFactory::make("videorate").build().unwrap();

            let capsfilter = ElementFactory::make("capsfilter")
                .property("caps", VideoCapsBuilder::new().build())
                .build()
                .unwrap();

            // TODO drop/max_buffers or qos don't really work right
            let appsink = AppSink::builder()
                .drop(true)
                // 1 second of 1080p at 30 fps ~= 8 MiB*30 = 240 MiB
                // 240*(10/30) = 80 MiB
                .max_buffers(10)
                .wait_on_eos(false)
                .caps(&VideoCapsBuilder::new().format(VideoFormat::Bgra).build())
                .build();

            Self {
                videorate,
                capsfilter,
                appsink,
            }
        }
    }

    impl ObjectImpl for Renderer {}

    impl PlayVideoRendererImpl for Renderer {
        fn create_video_sink(&self, play: &gstreamer_play::Play) -> gstreamer::Element {
            debug!(?play, "create_video_sink");

            let bin = Bin::with_name("renderer");

            let elements = [
                &self.videorate,
                &self.capsfilter,
                &self.appsink.clone().into(),
            ];
            bin.add_many(elements).unwrap();

            Element::link_many(elements).unwrap();

            for el in elements {
                el.sync_state_with_parent().unwrap();
            }

            let a = self.videorate.static_pad("sink").unwrap();
            let pad = GhostPad::with_target(&a).unwrap();
            bin.add_pad(&pad).unwrap();

            bin.upcast()
        }
    }
}

glib::wrapper! {
    pub struct Renderer(ObjectSubclass<imp::Renderer>) @implements PlayVideoRenderer;
}

impl Renderer {
    #[must_use]
    pub fn capsfilter(&self) -> &Element {
        &self.imp().capsfilter
    }

    #[must_use]
    pub fn videorate(&self) -> &Element {
        &self.imp().videorate
    }

    #[must_use]
    pub fn appsink(&self) -> &AppSink {
        &self.imp().appsink
    }
}

impl Default for Renderer {
    fn default() -> Renderer {
        glib::Object::new()
    }
}
