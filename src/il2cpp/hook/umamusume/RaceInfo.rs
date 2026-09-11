use crate::il2cpp::{symbols::{get_field_from_name, get_method_addr}, types::*};

static mut GET_RACETYPE_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_RaceType, GET_RACETYPE_ADDR, i32, this: *mut Il2CppObject);

static mut SET_RACETYPE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_RaceType, SET_RACETYPE_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut GET_COURSE_DISTANCE_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CourseDistance, GET_COURSE_DISTANCE_ADDR, i32, this: *mut Il2CppObject);

def_field_object_accessors!(get get__raceCourseSet, RACE_COURSE_SET_FIELD, Il2CppObject);
def_field_value_accessors!(get get__courseSetDistance, COURSE_SET_DISTANCE_FIELD, i32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceInfo);

    unsafe {
        GET_RACETYPE_ADDR = get_method_addr(RaceInfo, c"get_RaceType", 0);
        SET_RACETYPE_ADDR = get_method_addr(RaceInfo, c"set_RaceType", 1);
        GET_COURSE_DISTANCE_ADDR = get_method_addr(RaceInfo, c"get_CourseDistance", 0);
        RACE_COURSE_SET_FIELD = get_field_from_name(RaceInfo, c"<RaceCourseSet>k__BackingField");
    }

    get_class_or_return!(umamusume, Gallop, MasterRaceCourseSet);
    find_nested_class_or_return!(MasterRaceCourseSet, RaceCourseSet);

    unsafe {
        COURSE_SET_DISTANCE_FIELD = get_field_from_name(RaceCourseSet, c"Distance");
    }
}

/// Reads `RaceInfo.<RaceCourseSet>k__BackingField.Distance` — with fallback to
/// `RaceInfo.get_CourseDistance()` if the field chain is unavailable.
pub fn course_distance(race_info: *mut Il2CppObject) -> i32 {
    if race_info.is_null() {
        return 0;
    }
    let course_set = get__raceCourseSet(race_info);
    if !course_set.is_null() {
        let dist = get__courseSetDistance(course_set);
        if dist > 0 {
            return dist;
        }
    }
    if unsafe { GET_COURSE_DISTANCE_ADDR != 0 } {
        get_CourseDistance(race_info)
    } else {
        0
    }
}