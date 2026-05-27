use crate::{il2cpp::{ext::Il2CppObjectExt, symbols, types::*}};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CriAtomExPlayback {
    pub id: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AudioPlayback {
    pub criAtomExPlayback: CriAtomExPlayback,
    pub isError: bool,
    pub soundGroup: i32,
    pub is3dSound: bool,
    pub atomSourceListIndex: i32,
    pub cueSheetName: *mut crate::il2cpp::types::Il2CppString,
    pub cueName: *mut crate::il2cpp::types::Il2CppString,
    pub cueId: i32,
}

// Safety: every raw-pointer dereference in this function is guarded by a
// preceding null/zero check.  The function returns early on any missing
// symbol so that a bad game version can never reach a null call-site.
unsafe fn process_playback(
    playback: &mut AudioPlayback,
    audio_ctrl_dict: *mut Il2CppObject,
    target_time: f32
) {
    let dict_class = (*audio_ctrl_dict).klass();
    let get_item_method = crate::il2cpp::symbols::get_method_cached(dict_class, c"get_Item", 1)
        .unwrap_or(std::ptr::null());
    if get_item_method.is_null() { return; }

    let mut key = playback.soundGroup;
    let mut get_item_params: [*mut std::ffi::c_void; 1] = [&mut key as *mut _ as *mut std::ffi::c_void];
    let mut exc = std::ptr::null_mut();

    let audio_ctrl = crate::il2cpp::api::il2cpp_runtime_invoke(
        get_item_method, audio_ctrl_dict as _, get_item_params.as_mut_ptr(), &mut exc
    );
    if !exc.is_null() || audio_ctrl.is_null() { return; }

    // --- #3 fix: guard every get_field_from_name result before use ---
    let pool_field = crate::il2cpp::symbols::get_field_from_name((*audio_ctrl).klass(), c"pool");
    if pool_field.is_null() { return; }
    let pool = crate::il2cpp::symbols::get_field_object_value::<Il2CppObject>(audio_ctrl, pool_field);
    if pool.is_null() { return; }

    let source_list_field = crate::il2cpp::symbols::get_field_from_name((*pool).klass(), c"sourceList");
    if source_list_field.is_null() { return; }
    let source_list = crate::il2cpp::symbols::get_field_object_value::<Il2CppObject>(pool, source_list_field);
    if source_list.is_null() { return; }

    let Some(list) = crate::il2cpp::symbols::IList::<*mut Il2CppObject>::new(source_list) else { return; };
    let count = list.count();
    let mut cute_audio_source = std::ptr::null_mut();

    for i in 0..count {
        let obj = list.get(i).unwrap_or(std::ptr::null_mut());
        if !obj.is_null() {
            let is_same_method = crate::il2cpp::symbols::get_method_cached((*obj).klass(), c"IsSamePlaybackId", 1)
                .unwrap_or(std::ptr::null());
            if is_same_method.is_null() { continue; }

            let mut params: [*mut std::ffi::c_void; 1] = [playback as *mut _ as *mut std::ffi::c_void];
            let mut exc = std::ptr::null_mut();
            let res = crate::il2cpp::api::il2cpp_runtime_invoke(
                is_same_method, obj as _, params.as_mut_ptr(), &mut exc
            );
            if exc.is_null() && !res.is_null() {
                let is_same = crate::il2cpp::symbols::unbox::<bool>(res);
                if is_same {
                    cute_audio_source = obj;
                    break;
                }
            }
        }
    }

    if cute_audio_source.is_null() { return; }

    // --- #3 fix continued ---
    let source_list_field2 = crate::il2cpp::symbols::get_field_from_name((*cute_audio_source).klass(), c"sourceList");
    if source_list_field2.is_null() { return; }
    let source_list2 = crate::il2cpp::symbols::get_field_object_value::<Il2CppObject>(cute_audio_source, source_list_field2);

    let using_index_field = crate::il2cpp::symbols::get_field_from_name((*cute_audio_source).klass(), c"usingIndex");
    if using_index_field.is_null() { return; }
    let using_index = crate::il2cpp::symbols::get_field_value::<i32>(cute_audio_source, using_index_field);

    let Some(list2) = crate::il2cpp::symbols::IList::<*mut Il2CppObject>::new(source_list2) else { return; };
    let atom_source = list2.get(using_index).unwrap_or(std::ptr::null_mut());
    if atom_source.is_null() { return; }

    // --- #2 fix: check addr != 0 before every transmute+call ---
    let get_player_addr = crate::il2cpp::symbols::get_method_addr_cached((*atom_source).klass(), c"get_player", 0);
    if get_player_addr == 0 { return; }
    let get_player: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = std::mem::transmute(get_player_addr);
    let player = get_player(atom_source);
    if player.is_null() { return; }

    let stop_addr = crate::il2cpp::symbols::get_method_addr_cached((*player).klass(), c"StopWithoutReleaseTime", 0);
    if stop_addr != 0 {
        let stop: extern "C" fn(*mut Il2CppObject) = std::mem::transmute(stop_addr);
        stop(player);
    }

    let set_time_addr = crate::il2cpp::symbols::get_method_addr_cached((*player).klass(), c"SetStartTime", 1);
    if set_time_addr != 0 {
        let set_time: extern "C" fn(*mut Il2CppObject, i64) = std::mem::transmute(set_time_addr);
        let start_time_ms = (target_time * 1000.0).round() as i64;
        set_time(player, start_time_ms);
    }

    let start_method = crate::il2cpp::symbols::get_method_cached((*player).klass(), c"Start", 0)
        .unwrap_or(std::ptr::null());
    if start_method.is_null() { return; }

    let mut exc = std::ptr::null_mut();
    let res = crate::il2cpp::api::il2cpp_runtime_invoke(
        start_method, player as _, std::ptr::null_mut(), &mut exc
    );
    if exc.is_null() && !res.is_null() {
        let new_playback = crate::il2cpp::symbols::unbox::<CriAtomExPlayback>(res);

        let update_method = crate::il2cpp::symbols::get_method_cached((*player).klass(), c"Update", 1)
            .unwrap_or(std::ptr::null());
        if !update_method.is_null() {
            let mut params: [*mut std::ffi::c_void; 1] = [&new_playback as *const _ as *mut std::ffi::c_void];
            crate::il2cpp::api::il2cpp_runtime_invoke(update_method, player as _, params.as_mut_ptr(), &mut exc);
        }

        let pause_addr = crate::il2cpp::symbols::get_method_addr_cached((*player).klass(), c"Pause", 0);
        if pause_addr != 0 {
            let pause: extern "C" fn(*mut Il2CppObject) = std::mem::transmute(pause_addr);
            pause(player);
        }

        playback.criAtomExPlayback = new_playback;

        let set_playback_method = crate::il2cpp::symbols::get_method_cached((*atom_source).klass(), c"set_Playback", 1)
            .unwrap_or(std::ptr::null());
        if !set_playback_method.is_null() {
            let mut params2: [*mut std::ffi::c_void; 1] = [&new_playback as *const _ as *mut std::ffi::c_void];
            crate::il2cpp::api::il2cpp_runtime_invoke(set_playback_method, atom_source as _, params2.as_mut_ptr(), &mut exc);
        }
    }
}

pub fn move_live_playback(target_time: f32) {
    let image = match symbols::get_assembly_image(c"umamusume.dll") {
        Ok(img) => img,
        Err(_) => return,
    };

    let dir_class = match symbols::get_class(image, c"Gallop.Live", c"Director") {
        Ok(c) => c,
        Err(_) => return,
    };

    // --- #1 fix: SingletonLike::new() returns Option — handle None gracefully ---
    let director = match symbols::SingletonLike::new(dir_class) {
        Some(s) => s.instance(),
        None => {
            warn!("move_live_playback: Director singleton field not found");
            return;
        }
    };
    if director.is_null() { return; }

    // --- #2 fix: check addr before transmute ---
    let is_pause_live_addr = symbols::get_method_addr_cached(dir_class, c"IsPauseLive", 0);
    if is_pause_live_addr == 0 {
        warn!("move_live_playback: IsPauseLive not found");
        return;
    }
    let is_pause_live: extern "C" fn(*mut Il2CppObject) -> bool =
        unsafe { std::mem::transmute(is_pause_live_addr) };
    let was_paused = is_pause_live(director);

    // --- #3 fix: guard field lookup ---
    let live_current_time_field = symbols::get_field_from_name(dir_class, c"_liveCurrentTime");
    if !live_current_time_field.is_null() {
        symbols::set_field_value(director, live_current_time_field, &target_time);
    }

    // --- #2 fix: check addr before transmute ---
    let get_time_controller_addr = symbols::get_method_addr_cached(dir_class, c"get_LiveTimeController", 0);
    if get_time_controller_addr == 0 {
        warn!("move_live_playback: get_LiveTimeController not found");
        return;
    }
    let get_time_controller: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(get_time_controller_addr) };
    let time_controller = get_time_controller(director);

    if !time_controller.is_null() {
        let tc_class = unsafe { (*time_controller).klass() };
        if !was_paused {
            // --- #2 fix ---
            let pause_live_addr = symbols::get_method_addr_cached(tc_class, c"PauseLive", 0);
            if pause_live_addr != 0 {
                let pause_live: extern "C" fn(*mut Il2CppObject) =
                    unsafe { std::mem::transmute(pause_live_addr) };
                pause_live(time_controller);
            }
        }

        // --- #3 fix ---
        let elapsed_time_field = symbols::get_field_from_name(tc_class, c"_elapsedTime");
        if !elapsed_time_field.is_null() {
            symbols::set_field_value(time_controller, elapsed_time_field, &target_time);
        }

        // --- #2 fix ---
        let set_current_time_addr = symbols::get_method_addr_cached(tc_class, c"set_CurrentTime", 1);
        if set_current_time_addr != 0 {
            let set_current_time: extern "C" fn(*mut Il2CppObject, f32) =
                unsafe { std::mem::transmute(set_current_time_addr) };
            set_current_time(time_controller, target_time);
        }
    }

    // --- #1 fix: both unwrap() calls replaced ---
    let am_class = match symbols::get_class(image, c"Gallop", c"AudioManager") {
        Ok(c) => c,
        Err(_) => {
            warn!("move_live_playback: AudioManager class not found");
            // Still need to resume if we paused above
            if !time_controller.is_null() && !was_paused {
                let tc_class = unsafe { (*time_controller).klass() };
                let resume_addr = symbols::get_method_addr_cached(tc_class, c"ResumeLive", 0);
                if resume_addr != 0 {
                    let resume_live: extern "C" fn(*mut Il2CppObject) =
                        unsafe { std::mem::transmute(resume_addr) };
                    resume_live(time_controller);
                }
            }
            return;
        }
    };
    let audio_manager = match symbols::SingletonLike::new(am_class) {
        Some(s) => s.instance(),
        None => {
            warn!("move_live_playback: AudioManager singleton field not found");
            std::ptr::null_mut()
        }
    };

    if !audio_manager.is_null() {
        unsafe {
            let get_cri_audio_manager_addr = symbols::get_method_addr_cached(am_class, c"get_CriAudioManager", 0);
            if get_cri_audio_manager_addr != 0 {
                let get_cri_audio_manager: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
                    std::mem::transmute(get_cri_audio_manager_addr);
                let cri_audio_manager = get_cri_audio_manager(audio_manager);

                if !cri_audio_manager.is_null() {
                    // --- #3 fix ---
                    let audio_ctrl_dict_field = symbols::get_field_from_name(
                        (*cri_audio_manager).klass(), c"audioCtrlDict"
                    );
                    if audio_ctrl_dict_field.is_null() {
                        warn!("move_live_playback: audioCtrlDict field not found");
                    } else {
                        let audio_ctrl_dict = symbols::get_field_object_value::<Il2CppObject>(
                            cri_audio_manager, audio_ctrl_dict_field
                        );

                        if !audio_ctrl_dict.is_null() {
                            // --- #3 fix ---
                            let song_playback_field = symbols::get_field_from_name(am_class, c"_songPlayback");
                            if !song_playback_field.is_null() {
                                let mut song_playback = symbols::get_field_value::<AudioPlayback>(
                                    audio_manager, song_playback_field
                                );
                                // --- #6 fix: process_playback is unsafe; wrap call site ---
                                process_playback(&mut song_playback, audio_ctrl_dict, target_time);
                                symbols::set_field_value(audio_manager, song_playback_field, &song_playback);
                            }

                            // --- #3 fix ---
                            let song_chara_playbacks_field = symbols::get_field_from_name(
                                am_class, c"_songCharaPlaybacks"
                            );
                            if !song_chara_playbacks_field.is_null() {
                                let song_chara_playbacks = symbols::get_field_object_value::<
                                    crate::il2cpp::types::Il2CppArray
                                >(audio_manager, song_chara_playbacks_field);

                                // --- #4 fix: null-check before Array::from + as_slice ---
                                if !song_chara_playbacks.is_null() {
                                    let chara_playbacks = crate::il2cpp::symbols::Array::<AudioPlayback>::from(
                                        song_chara_playbacks
                                    );
                                    let slice = chara_playbacks.as_slice();
                                    for i in 0..slice.len() {
                                        process_playback(&mut slice[i], audio_ctrl_dict, target_time);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                log::warn!("move_live_playback: get_CriAudioManager missing, skipping audio sync");
            }
        }
    }

    // --- #2 fix: check addr before transmute ---
    if !time_controller.is_null() && !was_paused {
        let tc_class = unsafe { (*time_controller).klass() };
        let resume_live_addr = symbols::get_method_addr_cached(tc_class, c"ResumeLive", 0);
        if resume_live_addr != 0 {
            let resume_live: extern "C" fn(*mut Il2CppObject) =
                unsafe { std::mem::transmute(resume_live_addr) };
            resume_live(time_controller);
        }
    }
}
