use gstreamer::Caps;
use gstreamer_audio::AudioCapsBuilder;
use gstreamer_video::VideoCapsBuilder;

#[must_use]
pub fn any_caps_video() -> Caps {
    VideoCapsBuilder::new().any_features().build()
}

#[must_use]
pub fn any_caps_audio() -> Caps {
    AudioCapsBuilder::new().any_features().build()
}

#[must_use]
pub fn any_caps_video_audio() -> Caps {
    let mut caps = any_caps_video();
    {
        let caps = caps.make_mut();
        caps.append(any_caps_audio());
    }
    caps
}
