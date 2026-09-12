use android_logger::FilterBuilder;
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::File;

pub fn init(filter_level: log::LevelFilter, file_logging: bool) {
    // Always init Android logcat immediately — never block the main thread on
    // a FUSE/sdcardfs File::create, which can hang at cold start and cause ANR.
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(filter_level)
            .with_filter(
                FilterBuilder::new()
                    .filter_level(filter_level)
                    .filter_module("sqlparser", log::LevelFilter::Off)
                    .build()
            )
            .with_tag("Hachimi")
    );

    if !file_logging {
        return;
    }

    // Defer file open to a background thread. External storage (FUSE) may block
    // on open() at cold start — doing it here on the main thread caused ANR.
    // WriteLogger::init will silently fail if logcat already claimed the global
    // logger slot, but that's acceptable; logcat output is always available.
    let path = {
        let mut p = super::utils::get_game_dir();
        p.push("hachimi.log");
        p
    };

    std::thread::Builder::new()
        .name("log_file_init".into())
        .spawn(move || {
            // Retry for up to 10 s in case storage mounts slowly after boot.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                match File::create(&path) {
                    Ok(file) => {
                        let config = ConfigBuilder::new()
                            .set_target_level(LevelFilter::Error)
                            .add_filter_ignore_str("sqlparser")
                            .set_time_format_rfc3339()
                            .build();
                        // Silently ignore failure — logcat is always running.
                        let _ = WriteLogger::init(filter_level, config, file);
                        return;
                    }
                    Err(_) if std::time::Instant::now() < deadline => {
                        std::thread::sleep(std::time::Duration::from_millis(250));
                    }
                    Err(e) => {
                        log::warn!("log_file_init: gave up opening {:?}: {}", path, e);
                        return;
                    }
                }
            }
        })
        .ok();
}