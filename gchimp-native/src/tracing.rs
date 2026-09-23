use std::{env, fs::OpenOptions, panic};

use git_version_macro::git_version;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

// copied from bxt-rs
pub fn setup_logging_hooks() {
    let timer_fmt = tracing_subscriber::fmt::time::LocalTime::rfc_3339;

    // Only write the message to the terminal (skipping span arguments) so it's less spammy.
    let only_message = tracing_subscriber::fmt::format::debug_fn(|writer, field, value| {
        if field.name() == "message" {
            write!(writer, "{value:?}")
        } else {
            Ok(())
        }
    });
    let term_layer = tracing_subscriber::fmt::layer()
        .with_timer(timer_fmt())
        .fmt_fields(only_message.clone());

    // Disable ANSI colors on Windows as they don't work properly in the legacy console window.
    // https://github.com/tokio-rs/tracing/issues/445
    // #[cfg(windows)]
    let term_layer = term_layer.with_timer(timer_fmt()).with_ansi(false);

    let current_dir_path = env::current_exe().expect("cannot locate gchimp binary");
    const LOG_FILE_NAME: &str = "gchimp.log";
    let log_file_path = current_dir_path.with_file_name(LOG_FILE_NAME);

    let file_layer = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path.as_path())
        .ok()
        .map(|file| {
            tracing_subscriber::fmt::layer()
                .with_writer(file)
                .with_ansi(false)
                .with_timer(timer_fmt())
        });

    let level_filter = LevelFilter::DEBUG;

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,gchimp=debug,gchimp-native=debug"));

    tracing_subscriber::registry()
        .with(level_filter)
        .with(env_filter)
        .with(term_layer)
        .with(file_layer)
        .init();

    panic::set_hook(Box::new(move |panic_info| {
        error!("{}", panic_info);
    }));

    info!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        git_version!(
            args = ["--tags", "--always", "--dirty=-modified"],
            cargo_prefix = "cargo:",
            fallback = "unknown"
        )
    );
}
