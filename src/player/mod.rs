pub mod renderer;

use std::sync::Arc;

use futures::StreamExt;
use gstreamer::{
    element_error, glib::object::ObjectExt, Caps, ClockTime, FlowError, FlowSuccess, Fraction,
    ResourceError, Sample,
};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use gstreamer_play::{Play, PlayMessage, PlayState, PlayVideoRenderer};
use gstreamer_video::VideoInfo;
use tracing::{debug, info, warn};

use self::renderer::Renderer;
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct GStreamerPlayer {
    renderer: Renderer,
    play: Play,
}

impl GStreamerPlayer {
    pub fn new() -> Result<Self> {
        let renderer = Renderer::default();
        let play = Play::new(Some::<PlayVideoRenderer>(renderer.clone().into()));

        Ok(Self { renderer, play })
    }

    pub fn set_fps(&mut self, fps: u16) {
        let capsfilter = self.renderer.capsfilter();

        let caps: Caps = capsfilter.property("caps");
        debug!(?caps);
        let mut caps = caps.clone();
        caps.make_mut()
            .set("framerate", Fraction::new(fps.into(), 1));
        debug!(?caps);
        capsfilter.set_property("caps", caps);
    }

    pub fn set_resolution(&mut self, width: u16, height: u16) {
        let capsfilter = self.renderer.capsfilter();

        let caps: Caps = capsfilter.property("caps");
        debug!(?caps);
        let mut caps = caps.clone();
        caps.make_mut().set("width", i32::from(width));
        caps.make_mut().set("height", i32::from(height));

        debug!(?caps);
        capsfilter.set_property("caps", caps);
    }

    pub fn set_volume(&mut self, volume: f64) -> Result<()> {
        if volume < 0.0 {
            return Err(Error::InvalidVolume(volume));
        }

        self.play.set_volume(volume);

        Ok(())
    }

    pub fn on_frame<CALLBACK>(&self, on_frame: CALLBACK)
    where
        CALLBACK: Fn(&[u8], u16, u16) + Sync + Send + 'static,
    {
        fn handle_sample<CALLBACK>(
            appsink: &AppSink,
            sample: &Sample,
            on_frame: &CALLBACK,
        ) -> std::result::Result<FlowSuccess, FlowError>
        where
            CALLBACK: Fn(&[u8], u16, u16) + Sync + Send + 'static,
        {
            let video_info = sample
                .caps()
                .and_then(|caps| VideoInfo::from_caps(caps).ok())
                .ok_or_else(|| {
                    element_error!(
                        appsink,
                        ResourceError::Failed,
                        ("Failed to get video info from sample")
                    );

                    FlowError::NotNegotiated
                })?;

            let buffer = sample.buffer().unwrap();
            let map = buffer.map_readable().unwrap();
            let bytes = map.as_slice();

            let width = video_info.width().try_into().unwrap();
            let height = video_info.height().try_into().unwrap();

            if !bytes.is_empty() && width != 0 && height != 0 {
                on_frame(bytes, width, height);
            }

            Ok(FlowSuccess::Ok)
        }

        let appsink = self.renderer.appsink();

        let on_frame = Arc::new(on_frame);

        let on_frame2 = on_frame.clone();
        appsink.set_callbacks(
            AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| FlowError::Eos)?;
                    handle_sample(appsink, &sample, &*on_frame)
                })
                .new_preroll(move |appsink| {
                    let sample = appsink.pull_preroll().map_err(|_| FlowError::Eos)?;
                    handle_sample(appsink, &sample, &*on_frame2)
                })
                .build(),
        );
    }

    pub fn play(&self, urls: &[String]) -> Result<()> {
        self.play
            .set_uri(Some(urls.first().ok_or(Error::MissingUrl)?));
        self.play.play();

        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        self.play.pause();

        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.play.stop();

        Ok(())
    }

    pub fn seek(&mut self, seconds: u64) {
        self.play.seek(ClockTime::from_seconds(seconds));
    }

    pub async fn message_loop(&self) -> Result<()> {
        let mut stream = self.play.message_bus().stream();
        let result = async move {
            while let Some(msg) = stream.next().await {
                let msg = PlayMessage::parse(&msg)
                    .map_err(|error| Error::PlayMessageParse(error, msg))?;
                match msg {
                    PlayMessage::EndOfStream => {
                        debug!("EndOfStream");
                        break;
                    }
                    PlayMessage::Error { error, details: _ } => {
                        return Err(Error::PlayMessage(error));
                    }
                    PlayMessage::Buffering { percent } => {
                        warn!(?percent, "Buffering");
                    }
                    PlayMessage::StateChanged { state } => {
                        info!(?state, "StateChanged");
                        if state == PlayState::Stopped {
                            break;
                        }
                    }
                    _ => (),
                }
            }

            Ok(())
        }
        .await;

        self.play.stop();

        // Set the message bus to flushing to ensure that all pending messages are dropped and there
        // are no further references to the play instance.
        self.play.message_bus().set_flushing(true);

        result
    }
}
