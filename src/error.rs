#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("gstreamer::init(): {0}")]
    Init(#[source] gstreamer::glib::Error),

    #[error("ElementFactory::make().build(): {0}")]
    ElementFactoryMake(#[source] gstreamer::glib::BoolError),

    #[error("Bin.add(): {0}")]
    BinAdd(#[source] gstreamer::glib::BoolError),

    #[error("Element.link(): {0}")]
    ElementLink(#[source] gstreamer::glib::BoolError),

    #[error("Element.sync_state_with_parent(): {0}")]
    ElementSync(#[source] gstreamer::glib::BoolError),

    #[error("PlayMessage::Error: {0}")]
    PlayMessage(#[source] gstreamer::glib::Error),

    #[error("PlayMessage::parse(): {0}: {:?}", gstreamer::MessageRef::type_(.1))]
    PlayMessageParse(#[source] gstreamer::glib::BoolError, gstreamer::Message),

    #[error("Element::set_state(): {0}")]
    StateChange(#[from] gstreamer::StateChangeError),

    #[error("GStreamerPlayer::new(): missing url")]
    MissingUrl,

    #[error("invalid volume {0}")]
    InvalidVolume(f64),
}

pub type Result<T> = std::result::Result<T, Error>;

#[test]
fn test_error_element_factory() {
    use gstreamer::ElementFactory;

    gstreamer::init().expect("gstreamer::init");

    let e: Error = ElementFactory::make("non exist")
        .build()
        .map_err(Error::ElementFactoryMake)
        .expect_err("didn't error?");

    assert_eq!(
        format!("{}", e),
        "ElementFactory::make().build(): Failed to find element factory with name 'non exist' for \
         creating element"
    );
    assert!(
        format!("{:?}", e).starts_with(r#"ElementFactoryMake(BoolError { message: "Failed to find element factory with name 'non exist' for creating element", filename: ""#)
    );
    assert!(format!("{:#?}", e).starts_with(
        r#"ElementFactoryMake(
    BoolError {
        message: "Failed to find element factory with name 'non exist' for creating element",
        filename: ""#
    ));
}

#[test]
// TODO how to get set_state() to error?
// #[ignore]
fn test_error_pipeline_state() {
    use gstreamer::{
        glib::object::Cast,
        prelude::{ElementExt, GstBinExt},
        Bin, ElementFactory, Pipeline, State,
    };

    gstreamer::init().expect("gstreamer::init");
    let pipeline = Pipeline::default();

    let bin = ElementFactory::make("uridecodebin").build().unwrap();
    pipeline.add(&bin).unwrap();
    bin.sync_state_with_parent().unwrap();
    let bin = bin.downcast_ref::<Bin>().unwrap();

    let e: Error = bin
        .set_state(State::Playing)
        .expect_err("didn't error?")
        .into();

    assert_eq!(
        format!("{}", e),
        "Element::set_state(): Element failed to change its state"
    );
    assert_eq!(format!("{:?}", e), r#"StateChange(StateChangeError)"#);
    assert_eq!(
        format!("{:#?}", e),
        r#"StateChange(
    StateChangeError,
)"#
    );

    pipeline.set_state(State::Null).unwrap();
}

#[test]
fn test_error_play_message_parse() {
    use gstreamer::message::Eos;
    use gstreamer_play::PlayMessage;

    gstreamer::init().expect("gstreamer::init");

    let message = Eos::new();
    let e: Error = PlayMessage::parse(&message)
        .map_err(|error| Error::PlayMessageParse(error, message))
        .expect_err("didn't error?");

    assert_eq!(
        format!("{}", e),
        "PlayMessage::parse(): Invalid play message: Eos"
    );
}
