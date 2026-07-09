use std::{
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH}
};
use once_cell::sync::Lazy;
use discord_rich_presence::{activity::{Activity, ActivityType, Assets, Timestamps}, DiscordIpc, DiscordIpcClient};
use crate::core::{Error, Hachimi};
use crate::core::game::Region;

static DISCORD_CLIENT: Lazy<Mutex<Option<DiscordIpcClient>>> = Lazy::new(|| {
    Mutex::new(None)
});

pub fn start_rpc() -> Result<(), Error> {
    let mut client_guard = DISCORD_CLIENT.lock().unwrap_or_else(|e| e.into_inner());
    if client_guard.is_some() {
        return Ok(());
    }

    // Choose appropriate Discord App ID based on detected release.
    let h = Hachimi::instance();
    let client_id = if h.game.is_steam_release {
        match h.game.region {
            Region::Global => "1387281194043048067", // Global Steam app ID
            Region::Japan => "1387432222147219607",  // JP Steam app ID
            _ => "1440812697925980294", // fallback (current app id)
        }
    } else {
        // Non-Steam releases keep using the configured/default app id
        "1440812697925980294"
    };

    let mut client = DiscordIpcClient::new(client_id);
    client.connect().map_err(|e| Error::DiscordRpcError(e.to_string()))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_secs();

    let activity = Activity::new()
        .activity_type(ActivityType::Playing)
        .assets(Assets::new().large_image("icon"))
        .timestamps(Timestamps::new().start(now as i64));

    client.set_activity(activity)
        .map_err(|e| Error::DiscordRpcError(e.to_string()))?;
    *client_guard = Some(client);
    info!("Rich presence set");
    Ok(())
}

pub fn stop_rpc() -> Result<(), Error> {
    let mut client_guard = DISCORD_CLIENT.lock().unwrap_or_else(|e| e.into_inner());
    
    if let Some(mut client) = client_guard.take() {
        info!("Stopping Discord RPC");
        client.close().map_err(|e| Error::DiscordRpcError(e.to_string()))?;
    }
    Ok(())
}
