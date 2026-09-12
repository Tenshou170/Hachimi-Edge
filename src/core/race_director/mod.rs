//! Race Director telemetry state: the pure data/model layer for the broadcast-style race
//! HUD (timing tower / followed-Uma panel / win probability / predicted positions).
//!
//! No IL2CPP or egui dependency here - `crate::il2cpp::hook::umamusume::race_telemetry`
//! is the writer (reads IL2CPP fields once per game-thread frame and calls into this
//! module), `crate::core::gui::race_director_hud` is the reader (builds `egui` panels from
//! `telemetry()`/`field_rows()`). Ported from the race-director-plugin's `telemetry.rs` +
//! `hooks.rs` (see that project for the original derivation of the win-probability model
//! and pace-trace bucketing) with two behavioral changes made possible by infrastructure
//! this codebase already has: gate identity comes from `HorseData.GateNo` instead of a
//! `RaceHorseData.frame_order` read, and race liveness is
//! `RaceHorseManagerBase::is_race_active()` instead of a "recent telemetry" timeout guess.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::Mutex;

fn clock() -> &'static std::time::Instant {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now)
}

// ── followed gate (who the "Followed Uma" panel / pace graph tracks) ──────────────────
static TARGET_GATE: AtomicI32 = AtomicI32::new(1);
static MANUAL_FOLLOW: AtomicBool = AtomicBool::new(false);

pub fn followed_gate() -> i32 {
    TARGET_GATE.load(Ordering::Relaxed)
}

/// A manual row click (timing tower) - always wins and sticks for the rest of the race.
pub fn set_followed_gate(gate: i32) {
    MANUAL_FOLLOW.store(true, Ordering::Relaxed);
    if gate != TARGET_GATE.swap(gate, Ordering::Relaxed) {
        on_switch_follow();
    }
}

/// Called once per gate, the moment the collector identifies it as the player's own horse.
/// No-op if the player already manually followed a row this race.
pub fn auto_follow_own_horse(gate: i32) {
    if MANUAL_FOLLOW.load(Ordering::Relaxed) {
        return;
    }
    if gate != TARGET_GATE.swap(gate, Ordering::Relaxed) {
        on_switch_follow();
    }
}

// ── per-horse telemetry ────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, Default)]
pub struct HorseTelem {
    pub gate: i32,
    pub order: i32,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub distance: f32,
    pub spurt: bool,
    pub exhausted: bool,
    pub late_start: bool,
    pub fight: bool,
    pub leading: bool,
    pub blocked: bool,
    pub prev_order: i32,
    pub popularity: i32,
    pub running_style: i32,
    pub defeat: i32,
}

static TELEM: Mutex<Vec<(i32, HorseTelem)>> = Mutex::new(Vec::new());
static NAME_MAP: Mutex<Vec<(i32, String)>> = Mutex::new(Vec::new());
static TRAINER_MAP: Mutex<Vec<(i32, String, i64)>> = Mutex::new(Vec::new());
static FINISH_RANK: Mutex<Vec<(i32, i32)>> = Mutex::new(Vec::new());
static FINISH_NEXT: AtomicI32 = AtomicI32::new(0);
static PREV_POS: Mutex<Vec<(i32, i32)>> = Mutex::new(Vec::new());
static LAST_POS_MS: AtomicU64 = AtomicU64::new(0);

pub fn gate_name(gate: i32) -> String {
    NAME_MAP
        .lock()
        .unwrap()
        .iter()
        .find(|(g, _)| *g == gate)
        .map(|(_, n)| n.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("Gate {gate}"))
}

/// Cheap peek so the collector can skip decoding the name/trainer managed strings for a
/// gate it has already identified this race (a string decode every frame, for every
/// horse, would be wasted work - identity never changes mid-race).
pub fn is_gate_identified(gate: i32) -> bool {
    NAME_MAP.lock().unwrap().iter().any(|(g, _)| *g == gate)
}
pub fn is_trainer_known(gate: i32) -> bool {
    TRAINER_MAP.lock().unwrap().iter().any(|(g, _, _)| *g == gate)
}

/// Cache display name for `gate`, once per race (subsequent calls no-op).
pub fn identify_gate(gate: i32, name: String) {
    let mut names = NAME_MAP.lock().unwrap();
    if names.iter().any(|(g, _)| *g == gate) {
        return;
    }
    names.push((gate, name));
}

pub fn set_trainer(gate: i32, name: String, viewer_id: i64) {
    let mut trainers = TRAINER_MAP.lock().unwrap();
    if trainers.iter().any(|(g, _, _)| *g == gate) {
        return;
    }
    trainers.push((gate, name, viewer_id));
}

fn trainer_of(gate: i32) -> (String, i64) {
    TRAINER_MAP
        .lock()
        .unwrap()
        .iter()
        .find(|(g, _, _)| *g == gate)
        .map(|(_, n, v)| (n.clone(), *v))
        .unwrap_or_default()
}

/// Record this frame's telemetry for one horse, and (if it just crossed the finish line)
/// its live finish rank. `course` is the race's course distance in meters (0 if unknown).
pub fn record_horse(gate: i32, t: HorseTelem, course: f32) {
    {
        let mut buf = TELEM.lock().unwrap();
        if let Some(slot) = buf.iter_mut().find(|(g, _)| *g == gate) {
            slot.1 = t;
        } else {
            buf.push((gate, t));
        }
    }

    if course > 0.0 && t.distance >= course {
        let mut ranks = FINISH_RANK.lock().unwrap();
        if !ranks.iter().any(|(g, _)| *g == gate) {
            let rank = FINISH_NEXT.fetch_add(1, Ordering::Relaxed) + 1;
            ranks.push((gate, rank));
        }
    }
}

/// Reset all per-race buffers. Called when race liveness transitions false -> true.
// ── race liveness sub-states (set by race_telemetry on the game thread) ──────────────────
// These let showing() filter out the pre-race start-dash window and the post-race
// finished state without calling IL2CPP methods on the render thread (which caused the
// crash fixed in the previous commit).
static RACE_GATE_OPEN: AtomicBool = AtomicBool::new(false);
static RACE_FINISHED: AtomicBool = AtomicBool::new(false);

pub fn set_gate_open(v: bool) {
    RACE_GATE_OPEN.store(v, Ordering::Release);
}
pub fn is_gate_open() -> bool {
    RACE_GATE_OPEN.load(Ordering::Acquire)
}

pub fn set_race_finished(v: bool) {
    RACE_FINISHED.store(v, Ordering::Release);
}
pub fn is_race_finished() -> bool {
    RACE_FINISHED.load(Ordering::Acquire)
}

pub fn on_race_start() {
    RACE_GATE_OPEN.store(false, Ordering::Release);
    RACE_FINISHED.store(false, Ordering::Release);
    TELEM.lock().unwrap().clear();
    NAME_MAP.lock().unwrap().clear();
    TRAINER_MAP.lock().unwrap().clear();
    FINISH_RANK.lock().unwrap().clear();
    FINISH_NEXT.store(0, Ordering::Relaxed);
    PREV_POS.lock().unwrap().clear();
    TARGET_GATE.store(1, Ordering::Relaxed);
    MANUAL_FOLLOW.store(false, Ordering::Relaxed);
    PREDICTED.lock().unwrap().clear();
    reset_skill_feed();
    reset_pace();
}

fn on_switch_follow() {
    reset_skill_feed();
    reset_pace();
}

// ── course header ───────────────────────────────────────────────────────────────────────
static COURSE_DIST: AtomicI32 = AtomicI32::new(0);

pub fn set_course_distance(d: i32) {
    if d > 0 {
        COURSE_DIST.store(d, Ordering::Relaxed);
    }
}

pub fn course_distance() -> i32 {
    COURSE_DIST.load(Ordering::Relaxed)
}

// ── predicted (precomputed) race result - Japan only, see race_telemetry.rs ──────────────
#[derive(Clone, Copy)]
struct PredictedResult {
    order: i32,
    time: f32,
    defeat: i32,
}
static PREDICTED: Mutex<Vec<(i32, PredictedResult)>> = Mutex::new(Vec::new());

/// `gate` is 1-based; `order`/`time`/`defeat` are read straight off
/// `RaceSimulateHorseResultData` for `horseIndex = gate - 1` (confirmed live by the
/// race-director-plugin this was ported from: horseIndex == gate - 1 except in a genuine
/// photo finish, where the precomputed tie-break may not match the live crossing order).
pub fn set_predicted_result(gate: i32, order: i32, time: f32, defeat: i32) {
    PREDICTED
        .lock()
        .unwrap()
        .push((gate, PredictedResult { order, time, defeat }));
}

pub fn predicted_time(gate: i32) -> Option<f32> {
    PREDICTED
        .lock()
        .unwrap()
        .iter()
        .find(|(g, _)| *g == gate)
        .map(|(_, r)| r.time)
}

pub struct PredictedRow {
    pub gate: i32,
    pub order: i32,
    pub time: f32,
    pub defeat: i32,
}

/// Full precomputed result, sorted by predicted finish order (winner first). Empty until
/// (JP-only) the current race's result blob has been read.
pub fn predicted_rows() -> Vec<PredictedRow> {
    let mut out: Vec<PredictedRow> = PREDICTED
        .lock()
        .unwrap()
        .iter()
        .map(|(gate, r)| PredictedRow {
            gate: *gate,
            order: r.order,
            time: r.time,
            defeat: r.defeat,
        })
        .collect();
    out.sort_by_key(|r| r.order);
    out
}

// ── followed-Uma extras: skill feed / AI state / spurt outlook ───────────────────────────
#[derive(Clone, Copy, Default)]
pub struct FollowState {
    pub kakari: bool,
    pub temptation_mode: i32,
    pub keep_mode: i32,
    pub downhill: bool,
}
static FOLLOW_STATE: Mutex<FollowState> = Mutex::new(FollowState {
    kakari: false,
    temptation_mode: 0,
    keep_mode: 0,
    downhill: false,
});

pub fn set_follow_state(state: FollowState) {
    *FOLLOW_STATE.lock().unwrap() = state;
}
pub fn follow_state() -> FollowState {
    *FOLLOW_STATE.lock().unwrap()
}

static SPURT_OUTLOOK: AtomicI32 = AtomicI32::new(0);
pub fn set_spurt_outlook(v: i32) {
    SPURT_OUTLOOK.store(v, Ordering::Relaxed);
}
pub fn spurt_outlook() -> i32 {
    SPURT_OUTLOOK.load(Ordering::Relaxed)
}

static SKILL_FEED: Mutex<Vec<(i32, String, String)>> = Mutex::new(Vec::new());

/// The game's used-skill list only ever grows during a race - the caller (the IL2CPP
/// collector) diffs its length against this to know which entries are new, so this module
/// doesn't need its own dedup bookkeeping.
pub fn skill_feed_len() -> usize {
    SKILL_FEED.lock().unwrap().len()
}
pub fn push_skill_feed(id: i32, name: String, effect: String) {
    SKILL_FEED.lock().unwrap().push((id, name, effect));
}
pub fn skill_feed() -> Vec<(i32, String, String)> {
    SKILL_FEED.lock().unwrap().clone()
}
fn reset_skill_feed() {
    SKILL_FEED.lock().unwrap().clear();
}

// ── live speed history (followed Uma), sampled by race PROGRESS ──────────────────────────
pub const PACE_BUCKETS: usize = 140;
static SPEED_TRACE: Mutex<Vec<f32>> = Mutex::new(Vec::new());

pub fn push_pace(progress: f32, v: f32) {
    let b = ((progress.clamp(0.0, 1.0) * PACE_BUCKETS as f32) as usize).min(PACE_BUCKETS - 1);
    let mut t = SPEED_TRACE.lock().unwrap();
    while t.len() <= b {
        let fill = *t.last().unwrap_or(&v);
        t.push(fill);
    }
    t[b] = v;
}
pub fn speed_trace() -> Vec<f32> {
    SPEED_TRACE.lock().unwrap().clone()
}
fn reset_pace() {
    SPEED_TRACE.lock().unwrap().clear();
    SPURT_OUTLOOK.store(0, Ordering::Relaxed);
    *FOLLOW_STATE.lock().unwrap() = FollowState::default();
}

// ── computed views for the HUD ────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct TelemView {
    pub followed: HorseTelem,
    pub followed_name: String,
    pub rival: Option<HorseTelem>,
    pub rival_name: String,
    pub rival_ahead: bool,
    pub gap: f32,
}

pub fn telemetry() -> Option<TelemView> {
    let (followed, rival) = {
        let buf = TELEM.lock().unwrap();
        if buf.is_empty() {
            return None;
        }
        let target = followed_gate();
        let followed = buf.iter().find(|(g, _)| *g == target)?.1;
        let want_order = if followed.order > 1 { followed.order - 1 } else { followed.order + 1 };
        let rival = buf.iter().find(|(_, h)| h.order == want_order).map(|(_, h)| *h);
        (followed, rival)
    };
    let ahead = followed.order > 1;
    let gap = rival.map(|r| (r.distance - followed.distance).abs()).unwrap_or(0.0);
    let followed_name = gate_name(followed.gate);
    let rival_name = rival.map(|r| gate_name(r.gate)).unwrap_or_default();
    Some(TelemView {
        followed,
        followed_name,
        rival,
        rival_name,
        rival_ahead: ahead,
        gap,
    })
}

#[derive(Clone)]
pub struct FieldRow {
    pub pos: i32,
    pub gate: i32,
    pub name: String,
    pub style: i32,
    pub sta: f32,
    pub gap_leader: f32,
    pub trend: i32,
    pub popularity: i32,
    pub spurt: bool,
    pub exhausted: bool,
    pub fight: bool,
    pub blocked: bool,
    pub followed: bool,
    pub win: f32,
    pub distance: f32,
    pub speed: f32,
    pub trainer: String,
}

/// The full field for the broadcast timing tower, leader-first. Win-probability: grounded
/// in the true precomputed result when available (JP), softmax-sharpened by race progress;
/// otherwise a live-physics ETA heuristic ported verbatim from the race-director-plugin.
pub fn field_rows() -> Vec<FieldRow> {
    let mut hs: Vec<HorseTelem> = {
        let buf = TELEM.lock().unwrap();
        if buf.is_empty() {
            return Vec::new();
        }
        buf.iter().map(|(_, h)| *h).collect()
    };
    let target = followed_gate();
    let franks = FINISH_RANK.lock().unwrap().clone();
    let rank_of = |gate: i32| franks.iter().find(|(g, _)| *g == gate).map(|(_, r)| *r);
    hs.sort_by(|a, b| match (rank_of(a.gate), rank_of(b.gate)) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.distance.partial_cmp(&a.distance).unwrap_or(std::cmp::Ordering::Equal),
    });
    let leader_dist = hs.first().map(|h| h.distance).unwrap_or(0.0);
    let n = hs.len().max(1) as f32;

    let course = course_distance() as f32;
    let progress = if course > 0.0 { (leader_dist / course).clamp(0.0, 1.0) } else { 0.0 };
    let rbs = if course > 0.0 { 20.0 - (course - 2000.0) / 1000.0 } else { 20.0 };
    let gassed = (rbs * 0.85 + 0.3).max(1.0);
    let eta = |h: &HorseTelem| -> f32 {
        let remaining = if course > 0.0 { (course - h.distance).max(0.0) } else { 1.0 };
        if remaining <= 0.0 {
            return 0.0;
        }
        let spd = h.speed.max(rbs * 0.5).max(1.0);
        if h.exhausted || h.hp <= 1.0 {
            return remaining / gassed;
        }
        let gap = (spd - rbs + 12.0).max(0.0);
        let gmult = if progress >= 0.66 { 1.7 } else { 1.0 };
        let drain = (gap * gap / 144.0) * 20.0 * gmult;
        let t_at_pace = remaining / spd;
        let hp_needed = drain * t_at_pace;
        if h.hp >= hp_needed {
            remaining / spd
        } else {
            let t_survive = h.hp / drain.max(1e-3);
            let d_survive = (spd * t_survive).min(remaining);
            t_survive + (remaining - d_survive).max(0.0) / gassed
        }
    };

    let predicted: Vec<Option<f32>> = hs.iter().map(|h| predicted_time(h.gate)).collect();
    let logits: Vec<f32> = if predicted.iter().all(|p| p.is_some()) {
        let times: Vec<f32> = predicted.iter().map(|p| p.unwrap()).collect();
        let min_t = times.iter().cloned().fold(f32::MAX, f32::min);
        let k = 2.0 + 6.0 * progress * progress;
        times.iter().map(|t| -k * (t - min_t)).collect()
    } else {
        let etas: Vec<f32> = hs.iter().map(eta).collect();
        let min_eta = etas.iter().cloned().fold(f32::MAX, f32::min);
        hs.iter()
            .zip(&etas)
            .map(|(h, e)| {
                let pop = if h.popularity > 0 { h.popularity as f32 } else { n * 0.5 };
                let t = 0.8 + 5.0 * progress;
                -t * (e - min_eta) + 0.45 * (1.0 - progress) * -(pop - 1.0)
            })
            .collect()
    };
    let maxl = logits.iter().cloned().fold(f32::MIN, f32::max);
    let exps: Vec<f32> = logits.iter().map(|l| (l - maxl).exp()).collect();
    let sum: f32 = exps.iter().sum::<f32>().max(1e-6);

    let mut out = Vec::with_capacity(hs.len());
    for (i, h) in hs.iter().enumerate() {
        let (trainer, _viewer_id) = trainer_of(h.gate);
        out.push(FieldRow {
            pos: i as i32 + 1,
            gate: h.gate,
            name: gate_name(h.gate),
            style: h.running_style,
            sta: if h.max_hp > 0.0 { (h.hp / h.max_hp).clamp(0.0, 1.0) } else { 0.0 },
            gap_leader: (leader_dist - h.distance).max(0.0),
            trend: 0,
            popularity: h.popularity,
            spurt: h.spurt,
            exhausted: h.exhausted,
            fight: h.fight,
            blocked: h.blocked,
            followed: h.gate == target,
            win: exps[i] / sum,
            distance: h.distance,
            speed: h.speed,
            trainer,
        });
    }

    // Trend (position change vs a moment ago) is refreshed at most every 150ms rather
    // than every call - field_rows() is called from the render thread, up to once per
    // panel per frame, and without this the trend arrows would flicker every frame
    // instead of showing meaningful recent movement.
    let now = clock().elapsed().as_millis() as u64;
    let refresh = now.saturating_sub(LAST_POS_MS.load(Ordering::Relaxed)) > 150;
    let mut prev = PREV_POS.lock().unwrap();
    for r in out.iter_mut() {
        r.trend = prev.iter().find(|(g, _)| *g == r.gate).map(|(_, p)| p - r.pos).unwrap_or(0);
    }
    if refresh {
        for r in &out {
            match prev.iter_mut().find(|(g, _)| *g == r.gate) {
                Some(slot) => slot.1 = r.pos,
                None => prev.push((r.gate, r.pos)),
            }
        }
        LAST_POS_MS.store(now, Ordering::Relaxed);
    }
    out
}
