use android_logger::{AndroidLogger, FilterBuilder};
use log::{Log, Metadata, Record};
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::File;
use std::sync::OnceLock;

static FILE_LOGGER: OnceLock<Box<dyn Log>> = OnceLock::new();
static LOGGER_INIT: OnceLock<()> = OnceLock::new();

struct CombinedLogger {
    android: AndroidLogger,
}

impl Log for CombinedLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.android.enabled(metadata)
            || FILE_LOGGER.get().is_some_and(|f| f.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        self.android.log(record);
        if let Some(f) = FILE_LOGGER.get() {
            f.log(record);
        }
    }

    fn flush(&self) {
        self.android.flush();
        if let Some(f) = FILE_LOGGER.get() {
            f.flush();
        }
    }
}

pub fn init(filter_level: log::LevelFilter, file_logging: bool) {
    // Always init Android logcat immediately — never block the main thread on
    // a FUSE/sdcardfs File::create, which can hang at cold start and cause ANR.
    LOGGER_INIT.get_or_init(|| {
        let android = AndroidLogger::new(
            android_logger::Config::default()
                .with_max_level(filter_level)
                .with_filter(
                    FilterBuilder::new()
                        .filter_level(filter_level)
                        .filter_module("sqlparser", log::LevelFilter::Off)
                        .build(),
                )
                .with_tag("Hachimi"),
        );
        let _ = log::set_boxed_logger(Box::new(CombinedLogger { android }));
        log::set_max_level(filter_level);
    });

    if !file_logging || FILE_LOGGER.get().is_some() {
        return;
    }

    // Defer file open to a background thread. External storage (FUSE) may block
    // on open() at cold start — doing it here on the main thread caused ANR.
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
                        let _ = FILE_LOGGER.set(WriteLogger::new(filter_level, config, file));
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
