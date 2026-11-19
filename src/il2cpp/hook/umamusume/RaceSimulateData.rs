use crate::il2cpp::{
    symbols::{get_field_from_name, get_method_addr},
    types::*
};

def_method_wrapper_fn!(get_FrameDataList, GET_FRAME_DATA_LIST_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_field_object_accessors!(get__simEvDataList, set__simEvDataList, SIM_EV_DATA_LIST_FIELD, Il2CppObject);
def_field_object_accessors!(get get__horseResultDataArray, HORSE_RESULT_DATA_ARRAY_FIELD, Il2CppArray);

pub fn init(umamusume: *const Il2CppImage) {
    let race_simulate_data = crate::il2cpp::symbols::get_class(umamusume, c"StandaloneSimulator", c"RaceSimulateData")
        .or_else(|_| crate::il2cpp::symbols::get_class(umamusume, c"Gallop", c"RaceSimulateData"));

    let RaceSimulateData = match race_simulate_data {
        Ok(v) => v,
        Err(_) => return,
    };

    unsafe {
        GET_FRAME_DATA_LIST_ADDR = get_method_addr(RaceSimulateData, c"get_FrameDataList", 0);
        SIM_EV_DATA_LIST_FIELD = get_field_from_name(RaceSimulateData, c"_simEvDataList");
        HORSE_RESULT_DATA_ARRAY_FIELD = get_field_from_name(RaceSimulateData, c"_horseResultDataArray");
    }
}
