use crate::il2cpp::{
    hook::umamusume::SimulateEventType::SimulateEventType,
    symbols::get_field_from_name,
    types::*
};

pub mod DistanceData;

def_field_object_accessors!(get_distanceData, set_distanceData, DISTANCE_DATA_FIELD, Il2CppObject);
def_field_value_accessors!(get_type, set_type, TYPE_FIELD, SimulateEventType);
def_field_object_accessors!(get_param, set_param, PARAM_FIELD, Il2CppArray);
def_field_value_accessors!(get_frameTime, set_frameTime, FRAME_TIME_FIELD, f32);

pub fn init(umamusume: *const Il2CppImage) {
    let race_simulate_event_data = crate::il2cpp::symbols::get_class(umamusume, c"StandaloneSimulator", c"RaceSimulateEventData")
        .or_else(|_| crate::il2cpp::symbols::get_class(umamusume, c"Gallop", c"RaceSimulateEventData"));

    let RaceSimulateEventData = match race_simulate_event_data {
        Ok(v) => v,
        Err(_) => return,
    };

    unsafe {
        DISTANCE_DATA_FIELD = get_field_from_name(RaceSimulateEventData, c"distanceData");
        TYPE_FIELD = get_field_from_name(RaceSimulateEventData, c"type");
        PARAM_FIELD = get_field_from_name(RaceSimulateEventData, c"param");
        FRAME_TIME_FIELD = get_field_from_name(RaceSimulateEventData, c"frameTime");
    }

    DistanceData::init(RaceSimulateEventData);
}
