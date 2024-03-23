#![warn(clippy::pedantic)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod player;
pub mod utils;

use tracing::info;

pub use self::error::{Error, Result};

pub fn init() -> Result<()> {
    info!("gstreamer::init()");
    gstreamer::init().map_err(Error::Init)?;

    Ok(())
}

#[cfg(test)]
fn test_setup() -> crate::error::Result<()> {
    use std::env::{self};

    use tracing_subscriber::EnvFilter;

    let level = if true { "debug" } else { "info" };
    let my_crate_name = env!("CARGO_PKG_NAME").replace('-', "_");

    let filter = EnvFilter::from_default_env()
        .add_directive(format!("{my_crate_name}={level}").parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .without_time()
        .init();

    if env::var_os("GST_DEBUG").is_none() {
        // ERROR + WARNING
        env::set_var("GST_DEBUG", "2");
    }

    init()?;

    Ok(())
}

#[cfg(test)]
#[tokio::test]
#[ignore]
async fn test_main() -> crate::error::Result<()> {
    use std::{env::args, time::Duration};

    use tokio::time::sleep;
    use tracing::debug;

    use crate::player::GStreamerPlayer;

    test_setup()?;

    // `cargo test main -- $urls`
    let urls = args().skip(2).collect::<Vec<_>>();

    let mut player = GStreamerPlayer::new()?;
    player.set_fps(10);
    player.set_resolution(320, 180);
    player.set_volume(0.5)?;
    player.on_frame(move |bytes, width, height| {
        debug!(?width, ?height, bytes = ?&bytes[0..16], "on_frame");
    });

    let message_loop_task = {
        let player = player.clone();
        async move {
            player.message_loop().await?;
            Ok::<(), Error>(())
        }
    };

    let work_task = async move {
        player.play(&urls)?;

        sleep(Duration::from_secs(5)).await;

        player.set_fps(3);
        player.set_resolution(32, 18);
        player.set_volume(0.1)?;
        player.seek(20);

        sleep(Duration::from_secs(5)).await;

        player.stop()?;

        Ok::<(), Error>(())
    };

    tokio::pin!(message_loop_task);
    tokio::select! {
        result = &mut message_loop_task => {
            result?;
        }

        result = work_task => {
            result?;
            message_loop_task.await?;
        }
    };

    Ok(())
}
