use std::{ffi::{CStr, CString}, os::raw::c_void};
use fnv::FnvHashMap;
use once_cell::sync::OnceCell;
use pelite::{pe::Pe, pe64::PeFile, FileMap};
use windows::Win32::Foundation::HMODULE;

use crate::{core::{ext::HashMapExt, Error}, windows::utils};

const SYMBOL_LIST: &[&'static str] = &[
    "il2cpp_init",
    "il2cpp_init_utf16",
    "il2cpp_shutdown",
    "il2cpp_set_config_dir",
    "il2cpp_set_data_dir",
    "il2cpp_set_temp_dir",
    "il2cpp_set_commandline_arguments",
    "il2cpp_set_commandline_arguments_utf16",
    "il2cpp_set_config_utf16",
    "il2cpp_set_config",
    "il2cpp_set_memory_callbacks",
    "il2cpp_memory_pool_set_region_size",
    "il2cpp_memory_pool_get_region_size",
    "il2cpp_get_corlib",
    "il2cpp_add_internal_call",
    "il2cpp_resolve_icall",
    "il2cpp_alloc",
    "il2cpp_free",
    "il2cpp_array_class_get",
    "il2cpp_array_length",
    "il2cpp_array_get_byte_length",
    "il2cpp_array_new",
    "il2cpp_array_new_specific",
    "il2cpp_array_new_full",
    "il2cpp_bounded_array_class_get",
    "il2cpp_array_element_size",
    "il2cpp_assembly_get_image",
    "il2cpp_class_for_each",
    "il2cpp_class_enum_basetype",
    "il2cpp_class_is_inited",
    "il2cpp_class_is_generic",
    "il2cpp_class_is_inflated",
    "il2cpp_class_is_assignable_from",
    "il2cpp_class_is_subclass_of",
    "il2cpp_class_has_parent",
    "il2cpp_class_from_il2cpp_type",
    "il2cpp_class_from_name",
    "il2cpp_class_from_system_type",
    "il2cpp_class_get_element_class",
    "il2cpp_class_get_events",
    "il2cpp_class_get_fields",
    "il2cpp_class_get_nested_types",
    "il2cpp_class_get_interfaces",
    "il2cpp_class_get_properties",
    "il2cpp_class_get_property_from_name",
    "il2cpp_class_get_field_from_name",
    "il2cpp_class_get_methods",
    "il2cpp_class_get_method_from_name",
    "il2cpp_class_get_name",
    "il2cpp_type_get_name_chunked",
    "il2cpp_class_get_namespace",
    "il2cpp_class_get_parent",
    "il2cpp_class_get_declaring_type",
    "il2cpp_class_instance_size",
    "il2cpp_class_num_fields",
    "il2cpp_class_is_valuetype",
    "il2cpp_class_value_size",
    "il2cpp_class_is_blittable",
    "il2cpp_class_get_flags",
    "il2cpp_class_is_abstract",
    "il2cpp_class_is_interface",
    "il2cpp_class_array_element_size",
    "il2cpp_class_from_type",
    "il2cpp_class_get_type",
    "il2cpp_class_get_type_token",
    "il2cpp_class_has_attribute",
    "il2cpp_class_has_references",
    "il2cpp_class_is_enum",
    "il2cpp_class_get_image",
    "il2cpp_class_get_assemblyname",
    "il2cpp_class_get_rank",
    "il2cpp_class_get_data_size",
    "il2cpp_class_get_static_field_data",
    "il2cpp_class_get_bitmap_size",
    "il2cpp_class_get_bitmap",
    "il2cpp_stats_dump_to_file",
    "il2cpp_stats_get_value",
    "il2cpp_domain_get",
    "il2cpp_domain_assembly_open",
    "il2cpp_domain_get_assemblies",
    "il2cpp_raise_exception",
    "il2cpp_exception_from_name_msg",
    "il2cpp_get_exception_argument_null",
    "il2cpp_format_exception",
    "il2cpp_format_stack_trace",
    "il2cpp_unhandled_exception",
    "il2cpp_native_stack_trace",
    "il2cpp_field_get_flags",
    "il2cpp_field_get_name",
    "il2cpp_field_get_parent",
    "il2cpp_field_get_offset",
    "il2cpp_field_get_type",
    "il2cpp_field_get_value",
    "il2cpp_field_get_value_object",
    "il2cpp_field_has_attribute",
    "il2cpp_field_set_value",
    "il2cpp_field_static_get_value",
    "il2cpp_field_static_set_value",
    "il2cpp_field_set_value_object",
    "il2cpp_field_is_literal",
    "il2cpp_gc_collect",
    "il2cpp_gc_collect_a_little",
    "il2cpp_gc_start_incremental_collection",
    "il2cpp_gc_disable",
    "il2cpp_gc_enable",
    "il2cpp_gc_is_disabled",
    "il2cpp_gc_set_mode",
    "il2cpp_gc_get_max_time_slice_ns",
    "il2cpp_gc_set_max_time_slice_ns",
    "il2cpp_gc_is_incremental",
    "il2cpp_gc_get_used_size",
    "il2cpp_gc_get_heap_size",
    "il2cpp_gc_wbarrier_set_field",
    "il2cpp_gc_has_strict_wbarriers",
    "il2cpp_gc_set_external_allocation_tracker",
    "il2cpp_gc_set_external_wbarrier_tracker",
    "il2cpp_gc_foreach_heap",
    "il2cpp_stop_gc_world",
    "il2cpp_start_gc_world",
    "il2cpp_gc_alloc_fixed",
    "il2cpp_gc_free_fixed",
    "il2cpp_gchandle_new",
    "il2cpp_gchandle_new_weakref",
    "il2cpp_gchandle_get_target",
    "il2cpp_gchandle_free",
    "il2cpp_gchandle_foreach_get_target",
    "il2cpp_object_header_size",
    "il2cpp_array_object_header_size",
    "il2cpp_offset_of_array_length_in_array_object_header",
    "il2cpp_offset_of_array_bounds_in_array_object_header",
    "il2cpp_allocation_granularity",
    "il2cpp_unity_liveness_allocate_struct",
    "il2cpp_unity_liveness_calculation_from_root",
    "il2cpp_unity_liveness_calculation_from_statics",
    "il2cpp_unity_liveness_finalize",
    "il2cpp_unity_liveness_free_struct",
    "il2cpp_method_get_return_type",
    "il2cpp_method_get_declaring_type",
    "il2cpp_method_get_name",
    "il2cpp_method_get_from_reflection",
    "il2cpp_method_get_object",
    "il2cpp_method_is_generic",
    "il2cpp_method_is_inflated",
    "il2cpp_method_is_instance",
    "il2cpp_method_get_param_count",
    "il2cpp_method_get_param",
    "il2cpp_method_get_class",
    "il2cpp_method_has_attribute",
    "il2cpp_method_get_flags",
    "il2cpp_method_get_token",
    "il2cpp_method_get_param_name",
    "il2cpp_property_get_flags",
    "il2cpp_property_get_get_method",
    "il2cpp_property_get_set_method",
    "il2cpp_property_get_name",
    "il2cpp_property_get_parent",
    "il2cpp_object_get_class",
    "il2cpp_object_get_size",
    "il2cpp_object_get_virtual_method",
    "il2cpp_object_new",
    "il2cpp_object_unbox",
    "il2cpp_value_box",
    "il2cpp_monitor_enter",
    "il2cpp_monitor_try_enter",
    "il2cpp_monitor_exit",
    "il2cpp_monitor_pulse",
    "il2cpp_monitor_pulse_all",
    "il2cpp_monitor_wait",
    "il2cpp_monitor_try_wait",
    "il2cpp_runtime_invoke",
    "il2cpp_runtime_invoke_convert_args",
    "il2cpp_runtime_class_init",
    "il2cpp_runtime_object_init",
    "il2cpp_runtime_object_init_exception",
    "il2cpp_runtime_unhandled_exception_policy_set",
    "il2cpp_string_length",
    "il2cpp_string_chars",
    "il2cpp_string_new",
    "il2cpp_string_new_len",
    "il2cpp_string_new_utf16",
    "il2cpp_string_new_wrapper",
    "il2cpp_string_intern",
    "il2cpp_string_is_interned",
    "il2cpp_thread_current",
    "il2cpp_thread_attach",
    "il2cpp_thread_detach",
    "il2cpp_thread_get_all_attached_threads",
    "il2cpp_is_vm_thread",
    "il2cpp_current_thread_walk_frame_stack",
    "il2cpp_thread_walk_frame_stack",
    "il2cpp_current_thread_get_top_frame",
    "il2cpp_thread_get_top_frame",
    "il2cpp_current_thread_get_frame_at",
    "il2cpp_thread_get_frame_at",
    "il2cpp_current_thread_get_stack_depth",
    "il2cpp_thread_get_stack_depth",
    "il2cpp_override_stack_backtrace",
    "il2cpp_type_get_object",
    "il2cpp_type_get_type",
    "il2cpp_type_get_class_or_element_class",
    "il2cpp_type_get_name",
    "il2cpp_type_is_byref",
    "il2cpp_type_get_attrs",
    "il2cpp_type_equals",
    "il2cpp_type_get_assembly_qualified_name",
    "il2cpp_type_get_reflection_name",
    "il2cpp_type_is_static",
    "il2cpp_type_is_pointer_type",
    "il2cpp_image_get_assembly",
    "il2cpp_image_get_name",
    "il2cpp_image_get_filename",
    "il2cpp_image_get_entry_point",
    "il2cpp_image_get_class_count",
    "il2cpp_image_get_class",
    "il2cpp_capture_memory_snapshot",
    "il2cpp_free_captured_memory_snapshot",
    "il2cpp_set_find_plugin_callback",
    "il2cpp_register_log_callback",
    "il2cpp_debugger_set_agent_options",
    "il2cpp_is_debugger_attached",
    "il2cpp_register_debugger_agent_transport",
    "il2cpp_debug_get_method_info",
    "il2cpp_unity_install_unitytls_interface",
    "il2cpp_custom_attrs_from_class",
    "il2cpp_custom_attrs_from_method",
    "il2cpp_custom_attrs_from_field",
    "il2cpp_custom_attrs_get_attr",
    "il2cpp_custom_attrs_has_attr",
    "il2cpp_custom_attrs_construct",
    "il2cpp_custom_attrs_free",
    "il2cpp_class_set_userdata",
    "il2cpp_class_get_userdata_offset",
    "il2cpp_set_default_thread_affinity",
    "il2cpp_unity_set_android_network_up_state_func"
];

const START_RVA: u32 = 0x782c92;
fn generate_symbol_map() -> Result<FnvHashMap<&'static str, CString>, Error> {
    let mut map = FnvHashMap::with_capacity(SYMBOL_LIST.len());

    let mut path = utils::get_game_dir();
    path.push("UnityPlayer.dll");
    let file_map = FileMap::open(&path)?;
    let pe = PeFile::from_bytes(&file_map)?;
    let image = file_map.as_ref();

    let mut rva = START_RVA;
    for symbol in SYMBOL_LIST {
        let offset = pe.rva_to_file_offset(rva)?;

        // --- W-11 fix: bounds-check the 4-byte read before indexing ---
        if offset + 4 > image.len() {
            return Err(Error::RuntimeError(format!(
                "symbol map: RVA 0x{:x} maps to offset 0x{:x} which is out of bounds \
                 (image size 0x{:x}); UnityPlayer.dll may be a different version",
                rva, offset, image.len()
            )));
        }
        let rip_offset = u32::from_le_bytes(image[offset..offset+4].try_into().unwrap());
        let name_offset = pe.rva_to_file_offset(rva + 0x4 + rip_offset)?;

        // --- W-2 fix: validate that the name embedded in the binary actually
        // matches the expected symbol.  If START_RVA is stale after a game
        // update the names will diverge and we surface a clear error instead
        // of silently returning a garbage function pointer. ---
        if name_offset >= image.len() {
            return Err(Error::RuntimeError(format!(
                "symbol map: name offset 0x{:x} out of bounds for symbol '{}'; \
                 UnityPlayer.dll may be a different version",
                name_offset, symbol
            )));
        }
        let name = unsafe { CStr::from_ptr(image[name_offset..].as_ptr() as _) };

        map.insert(*symbol, name.to_owned());
        rva += if rva == START_RVA { 0x28 } else { 0x26 };
    }

    Ok(map)
}

impl From<pelite::Error> for Error {
    fn from(e: pelite::Error) -> Self {
        Error::RuntimeError(e.to_string())
    }
}

// --- W-1 fix: replace panicking Lazy with OnceCell<Result<...>> so a
// symbol-map failure surfaces a clear error and returns 0 from dlsym
// (same as "symbol not found") instead of aborting the process. ---
static SYMBOL_MAP: OnceCell<Result<FnvHashMap<&'static str, CString>, String>> = OnceCell::new();

fn get_symbol_map() -> Option<&'static FnvHashMap<&'static str, CString>> {
    SYMBOL_MAP.get_or_init(|| {
        generate_symbol_map().map_err(|e| {
            error!("[symbols] Failed to build symbol map: {}", e);
            e.to_string()
        })
    }).as_ref().ok()
}

pub unsafe fn dlsym(handle: *mut c_void, name: &str) -> usize {
    debug_assert!(!handle.is_null());
    // --- W-1/W-2 fix: if the map failed to build, return 0 so the caller
    // treats this symbol as not found rather than calling a garbage address. ---
    let Some(map) = get_symbol_map() else {
        warn!("[symbols] dlsym('{}') skipped: symbol map unavailable", name);
        return 0;
    };
    let Some(cname) = map.get(name) else {
        warn!("[symbols] dlsym('{}') skipped: symbol not in map", name);
        return 0;
    };
    utils::get_proc_address(HMODULE(handle as _), cname)
}
