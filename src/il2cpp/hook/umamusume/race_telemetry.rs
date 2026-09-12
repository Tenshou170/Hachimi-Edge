//! IL2CPP glue for the Race Director HUD (`crate::core::race_director`, drawn by
//! `crate::core::gui::race_director_hud`). Ported from the race-director-plugin's
//! `hooks.rs` + `telemetry.rs`, but collected by polling once per game-thread frame
//! (`collect_frame`, called from `GameSystem_Update`) instead of hooking a per-horse
//! method - see `crate::core::race_director`'s module doc for why.
//!
//! Every field/method this reads was already resolved by the relevant per-class module's
//! own `init()` (`HorseData`, `HorseRaceInfo`, `RaceHorseData`, `RaceInfo`,
//! `RaceHorseManagerBase`, `RaceManager`, `RaceSimulateData`, `RaceSimulateReader`,
//! `RaceSimulateHorseResultData`, `SkillManager`, `MasterDataManager`, `MasterDataUtil`,
//! `MasterSkillData`) - this module only orchestrates, it doesn't do its own class/field
//! resolution.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::{game::Region, race_director, Hachimi};
use crate::il2cpp::{
    ext::Il2CppStringExt,
    symbols::{Array, IList},
    types::*,
};

use super::{
    HorseData, HorseRaceAIBase, HorseRaceAIReplay, HorseRaceInfo, MasterDataManager,
    MasterDataUtil, MasterSkillData, RaceHorseData, RaceHorseManagerBase, RaceHorseManagerReplay,
    RaceInfo, RaceManager, RaceSimulateData, RaceSimulateHorseResultData, RaceSimulateReader,
    SkillManager,
};

fn string_of(s: *mut Il2CppString) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe { (*s).as_utf16str().to_string() }
}

fn is_japan() -> bool {
    Hachimi::instance().game.region == Region::Japan
}

fn is_replay_sim_supported() -> bool {
    matches!(Hachimi::instance().game.region, Region::Japan | Region::Global)
}

static WAS_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Called unconditionally from `GameSystem_Update`, every game-thread frame, on both
/// platforms. Cheap no-op outside an active race.
pub fn collect_frame() {
    if !Hachimi::instance().config.load().race_director.enabled {
        return;
    }

    if !RaceHorseManagerBase::is_race_active() {
        WAS_ACTIVE.store(false, Ordering::Relaxed);
        return;
    }

    let race_manager = RaceManager::instance();
    if race_manager.is_null() {
        return;
    }
    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() {
        return;
    }

    if !WAS_ACTIVE.swap(true, Ordering::Relaxed) {
        race_director::on_race_start();
        // Each isolated behind its own catch_unwind: these two run exactly once, at the
        // exact moment a race starts, and are the least battle-tested part of this
        // feature (RaceInfo's RaceCourseSet/Distance field chain and the whole
        // predicted-result/MasterSkillData chain have no proven-working precedent
        // elsewhere in this codebase, unlike the per-horse field reads below). Isolating
        // them means a fault in one doesn't also take out the other, and pins down
        // exactly which one is at fault in the log if either panics.
        if std::panic::catch_unwind(move || read_course_distance(race_manager)).is_err() {
            error!("race_telemetry: read_course_distance PANICKED (caught)");
        }
        if std::panic::catch_unwind(move || read_predicted_result(horse_manager)).is_err() {
            error!("race_telemetry: read_predicted_result PANICKED (caught)");
        }
    }

    let horse_infos = RaceHorseManagerBase::GetHorseRaceInfos(horse_manager);
    if horse_infos.is_null() {
        return;
    }

    // Update gate-open and finished state on the game thread where it's safe to call IL2CPP.
    // is_start_dash_instance checks whether the pre-race countdown is still running;
    // once it returns false the gate has opened and the race is properly underway.
    // IsFinished is polled the same way to dismiss the HUD once the race concludes.
    if !race_director::is_gate_open() {
        if !HorseRaceInfo::is_start_dash_instance(race_manager) {
            race_director::set_gate_open(true);
        }
    }
    if !race_director::is_race_finished() {
        if HorseRaceInfo::is_finished() {
            race_director::set_race_finished(true);
        }
    }
    let arr: Array<*mut Il2CppObject> = Array::from(horse_infos);
    let course = race_director::course_distance() as f32;
    let target = race_director::followed_gate();
    let japan = is_japan();

    for info in unsafe { arr.as_slice() }.iter().copied() {
        if info.is_null() {
            continue;
        }
        let result = std::panic::catch_unwind(move || read_horse(info));
        let Ok(Some((gate, telem))) = result else {
            if result.is_err() {
                error!("race_telemetry: read_horse PANICKED (caught)");
            }
            continue;
        };
        race_director::record_horse(gate, telem, course);

        if gate == target
            && std::panic::catch_unwind(move || update_followed(info, course, telem, japan)).is_err()
        {
            error!("race_telemetry: update_followed PANICKED (caught)");
        }
    }
}

fn read_course_distance(race_manager: *mut Il2CppObject) {
    let race_info = RaceManager::get_RaceInfo(race_manager);
    let dist = RaceInfo::course_distance(race_info);
    race_director::set_course_distance(dist);
}

/// horseIndex == gate - 1 (confirmed live by the race-director-plugin this was ported
/// from) - see `RaceSimulateHorseResultData`'s doc.
fn read_predicted_result(horse_manager: *mut Il2CppObject) {
    if !is_replay_sim_supported() || !RaceHorseManagerReplay::is_replay_manager(horse_manager) {
        return;
    }
    let reader = RaceHorseManagerReplay::get__reader(horse_manager);
    if reader.is_null() {
        return;
    }
    let sim_data = RaceSimulateReader::get__simData(reader);
    if sim_data.is_null() {
        return;
    }
    let result_array = RaceSimulateData::get__horseResultDataArray(sim_data);
    if result_array.is_null() {
        return;
    }
    let arr: Array<*mut Il2CppObject> = Array::from(result_array);
    for (i, elem) in unsafe { arr.as_slice() }.iter().copied().enumerate() {
        if elem.is_null() {
            continue;
        }
        let gate = i as i32 + 1;
        let order = RaceSimulateHorseResultData::get__finishOrder(elem);
        let time = RaceSimulateHorseResultData::get__finishTime(elem);
        let defeat = RaceSimulateHorseResultData::get__defeat(elem);
        race_director::set_predicted_result(gate, order, time, defeat);
    }
}

fn read_horse(info: *mut Il2CppObject) -> Option<(i32, race_director::HorseTelem)> {
    let hdata = HorseRaceInfo::get__horseData(info);
    if hdata.is_null() {
        return None;
    }
    let gate = HorseData::get_GateNo(hdata);
    if gate <= 0 {
        return None;
    }

    let resp = HorseData::get__responseHorseData(hdata);
    identify_and_maybe_follow(gate, hdata, resp);

    let phase = HorseRaceInfo::get__phase(info);
    let running_style = if resp.is_null() { 0 } else { RaceHorseData::get__runningStyle(resp) };

    let telem = race_director::HorseTelem {
        gate,
        order: HorseRaceInfo::get__curOrder(info),
        hp: HorseRaceInfo::get__hp(info),
        max_hp: HorseRaceInfo::get__maxHp(info),
        speed: HorseRaceInfo::get__lastSpeed(info),
        distance: HorseRaceInfo::get__distance(info),
        spurt: phase >= 2,
        exhausted: HorseRaceInfo::get__isHpEmptyOnRace(info),
        late_start: HorseRaceInfo::get__isBadStart(info),
        fight: HorseRaceInfo::get__isCompeteFight(info),
        leading: HorseRaceInfo::get__isCompeteTop(info),
        // No pure-field source for "boxed in" exists (confirmed against a full JP il2cpp
        // dump by the race-director-plugin this was ported from - only a virtual method,
        // HorseRaceInfo.IsBlockFront(), that computes it live against neighbours). Left
        // false rather than add a per-horse-per-frame method call for a cosmetic tag.
        blocked: false,
        prev_order: HorseRaceInfo::get__prevOrder(info),
        popularity: HorseData::get__Popularity(hdata),
        running_style,
        defeat: HorseData::get__Defeat(hdata),
    };

    Some((gate, telem))
}

fn identify_and_maybe_follow(gate: i32, hdata: *mut Il2CppObject, resp: *mut Il2CppObject) {
    if !race_director::is_gate_identified(gate) {
        let name = string_of(HorseData::get__charaName(hdata));
        race_director::identify_gate(gate, name);

        if HorseData::get_IsUser(hdata) {
            race_director::auto_follow_own_horse(gate);
        }
    }

    if !resp.is_null() && !race_director::is_trainer_known(gate) {
        let trainer = string_of(RaceHorseData::get__trainerName(resp));
        let viewer_id = RaceHorseData::get__viewerId(resp);
        race_director::set_trainer(gate, trainer, viewer_id);
    }
}

/// Followed-Uma-only extras: pace trace, AI state tags, spurt outlook, skill feed.
fn update_followed(info: *mut Il2CppObject, course: f32, telem: race_director::HorseTelem, japan: bool) {
    if course > 0.0 && telem.speed > 0.5 {
        race_director::push_pace(telem.distance / course, telem.speed);
    }

    let ai = HorseRaceInfo::get__horseRaceAI(info);
    if !ai.is_null() {
        let kakari = HorseRaceAIBase::get_IsTemptation(ai);
        let temptation_mode = HorseRaceAIBase::get_TemptationMode(ai);
        let keep_mode = HorseRaceAIBase::get_PositionKeepMode(ai);
        let downhill = HorseRaceAIBase::get_IsDownSlopeAccelMode(ai);
        race_director::set_follow_state(race_director::FollowState { kakari, temptation_mode, keep_mode, downhill });
        if telem.spurt {
            race_director::set_spurt_outlook(HorseRaceAIReplay::get_LastSpurtCalcResult(ai));
        }
    }

    // Skill activation data (SkillManager.GetUsedSkillIdList) only resolves on Japan -
    // see SkillManager.rs's own Region::Japan init gate.
    if japan {
        update_skill_feed(info);
    }
}

fn update_skill_feed(info: *mut Il2CppObject) {
    let mgr = HorseRaceInfo::get__skillManager(info);
    if mgr.is_null() {
        return;
    }
    let list_obj = SkillManager::GetUsedSkillIdList(mgr);
    let Some(list) = IList::<i32>::new(list_obj) else {
        return;
    };
    let seen = race_director::skill_feed_len();
    let count = list.count().max(0) as usize;
    if count <= seen || count > 64 {
        return;
    }
    for i in seen as i32..count as i32 {
        let Some(id) = list.get(i) else { continue };
        let name = skill_name(id);
        let effect = skill_effect(id);
        race_director::push_skill_feed(id, name, effect);
    }
}

fn skill_name(id: i32) -> String {
    let raw = string_of(MasterDataUtil::GetSkillName(id));
    if raw.is_empty() {
        return format!("Skill {id}");
    }
    // Trims trailing junk characters seen on some builds - ported verbatim from the
    // race-director-plugin's own live JP testing.
    let cleaned = raw.trim_end_matches(|c: char| c == ' ' || (c as u32) > 0x024F);
    if cleaned.is_empty() {
        raw
    } else {
        cleaned.to_string()
    }
}

fn skill_effect(id: i32) -> String {
    let mgr = MasterDataManager::instance();
    if mgr.is_null() {
        return String::new();
    }
    let msd = MasterDataManager::get__masterSkillData(mgr);
    if msd.is_null() {
        return String::new();
    }
    let sd = MasterSkillData::Get(msd, id);
    if sd.is_null() {
        return String::new();
    }
    let atype = MasterSkillData::get__abilityType11(sd);
    let aval = MasterSkillData::get__floatAbilityValue11(sd);
    let atime = MasterSkillData::get__floatAbilityTime1(sd);
    if atype == 0 || aval == 0 {
        return String::new();
    }
    let v = aval as f32 / 10000.0;
    let t = atime as f32 / 10000.0;
    let dur = if t > 0.4 { format!(" {t:.1}s") } else { String::new() };
    match atype {
        1 => format!("+{v:.0} Speed"),
        2 => format!("+{v:.0} Stamina"),
        3 => format!("+{v:.0} Power"),
        4 => format!("+{v:.0} Guts"),
        5 => format!("+{v:.0} Wisdom"),
        21 | 22 | 27 => format!("+{v:.2} m/s{dur}"),
        31 => format!("+{v:.2} m/s2{dur}"),
        9 => format!("+{:.0}% HP", v * 100.0),
        _ => format!("+{v:.2}{dur}"),
    }
}
