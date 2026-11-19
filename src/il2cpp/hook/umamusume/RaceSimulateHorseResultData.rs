use crate::il2cpp::{
    symbols::get_field_from_name,
    types::*
};

def_field_value_accessors!(get get__finishOrder, FINISH_ORDER_FIELD, i32);
def_field_value_accessors!(get get__finishTime, FINISH_TIME_FIELD, f32);
def_field_value_accessors!(get get__finishTimeRaw, FINISH_TIME_RAW_FIELD, f32);
def_field_value_accessors!(get get__finishDiffTime, FINISH_DIFF_TIME_FIELD, f32);
def_field_value_accessors!(get get__defeat, DEFEAT_FIELD, i32);

pub fn init(umamusume: *const Il2CppImage) {
    let race_simulate_horse_result_data = crate::il2cpp::symbols::get_class(umamusume, c"StandaloneSimulator", c"RaceSimulateHorseResultData")
        .or_else(|_| crate::il2cpp::symbols::get_class(umamusume, c"Gallop", c"RaceSimulateHorseResultData"));

    let RaceSimulateHorseResultData = match race_simulate_horse_result_data {
        Ok(v) => v,
        Err(_) => return,
    };

    unsafe {
        FINISH_ORDER_FIELD = get_field_from_name(RaceSimulateHorseResultData, c"FinishOrder");
        FINISH_TIME_FIELD = get_field_from_name(RaceSimulateHorseResultData, c"FinishTime");
        FINISH_TIME_RAW_FIELD = get_field_from_name(RaceSimulateHorseResultData, c"FinishTimeRaw");
        FINISH_DIFF_TIME_FIELD = get_field_from_name(RaceSimulateHorseResultData, c"FinishDiffTime");
        DEFEAT_FIELD = get_field_from_name(RaceSimulateHorseResultData, c"Defeat");
    }
}
