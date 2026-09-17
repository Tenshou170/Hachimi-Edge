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
    {
        let client_guard = DISCORD_CLIENT.lock().unwrap_or_else(|e| e.into_inner());
        if client_guard.is_some() {
            return Ok(());
        }
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

    // Connect with timeout to prevent hanging forever on broken pipes (e.g. Proton discord bridge)
    let (tx, rx) = std::sync::mpsc::channel();
    let thread_spawn = std::thread::Builder::new()
        .name("discord_ipc_conn".into())
        .spawn(move || {
            let res = client.connect().map(|_| client);
            let _ = tx.send(res);
        });

    if let Err(e) = thread_spawn {
        return Err(Error::DiscordRpcError(e.to_string()));
    }

    let mut client = match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => return Err(Error::DiscordRpcError(e.to_string())),
        Err(_) => return Err(Error::DiscordRpcError("Discord IPC connection timed out".to_string())),
    };

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

    let mut client_guard = DISCORD_CLIENT.lock().unwrap_or_else(|e| e.into_inner());
    *client_guard = Some(client);
    info!("Rich presence set");
    Ok(())
}

#[allow(dead_code)]
pub fn update_activity(
    details: Option<&str>,
    state: Option<&str>,
    large_image: Option<&str>,
    large_text: Option<&str>,
    start_timestamp: Option<i64>,
) -> Result<(), Error> {
    let mut client_guard = DISCORD_CLIENT.lock().unwrap_or_else(|e| e.into_inner());
    let Some(client) = client_guard.as_mut() else {
        return Ok(());
    };

    let mut activity = Activity::new().activity_type(ActivityType::Playing);
    if let Some(d) = details {
        activity = activity.details(d);
    }
    if let Some(s) = state {
        activity = activity.state(s);
    }

    let mut assets = Assets::new().large_image(large_image.unwrap_or("icon"));
    if let Some(lt) = large_text {
        assets = assets.large_text(lt);
    }
    activity = activity.assets(assets);

    if let Some(ts) = start_timestamp {
        activity = activity.timestamps(Timestamps::new().start(ts));
    }

    client.set_activity(activity)
        .map_err(|e| Error::DiscordRpcError(e.to_string()))?;
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
