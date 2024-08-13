use ash::ext::debug_utils;
use ash::{vk, Entry, Instance};
use std::{borrow::Cow, error::Error, ffi::CStr, ffi::CString, os::raw::c_void};

// This doesn't exist in this version of Ash
// const REQUIRED_LAYERS: [&'static str; 1] = ["VK_LAYER_LUNARG_standard_validation"];
// But this does
pub const REQUIRED_LAYERS: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

pub unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    // let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("") // Why do we use Cow again?
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("") // Why do we use Cow again?
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE {
        log::debug!(
            "{:?} {:?}: {} {}",
            message_severity,
            message_types,
            message_id_name,
            message,
        )
    } else if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        log::info!(
            "{:?} {:?}: ({}) {}",
            message_severity,
            message_types,
            message_id_name,
            message,
        )
    } else if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        log::warn!(
            "{:?} {:?}: {} {}",
            message_severity,
            message_types,
            message_id_name,
            message,
        )
    } else {
        log::error!(
            "{:?} {:?}: {} {}",
            message_severity,
            message_types,
            message_id_name,
            message,
        )
    }

    vk::FALSE
}

/// Get the pointers to the validation layers names.
/// Also return the corresponding `CString` to avoid dangling pointers.
pub fn get_layer_names_and_pointers() -> (Vec<CString>, Vec<*const i8>) {
    let layer_names = REQUIRED_LAYERS
        .iter()
        .map(|name| CString::new(*name).expect("Failed to build CString"))
        .collect::<Vec<_>>();

    let layer_names_ptrs = layer_names
        .iter()
        .map(|name| name.as_ptr())
        .collect::<Vec<_>>();

    (layer_names, layer_names_ptrs)
}

/// Check if the required validation set in `REQUIRED_LAYERS`
/// are supported by the Vulkan instance.
///
/// # Panics
///
/// Panic if at least one on the layer is not supported.
pub fn check_validation_layer_support(entry: &Entry) {
    for required in REQUIRED_LAYERS.iter() {
        log::info!("Searching for {:?}", required);
        unsafe {
            let found = entry
                .enumerate_instance_layer_properties()
                .unwrap()
                .iter()
                .any(|layer| {
                    let name = CStr::from_ptr(layer.layer_name.as_ptr());
                    let name = name.to_str().expect("Failed to get layer name pointer");
                    if required == &name {
                        log::info!("Found {:?}", name);
                        true
                    } else {
                        false
                    }
                });
            if !found {
                panic!("Validation layer not supported: {}", required);
            }
        };
    }
}

/// Setup the debug message if validation layers are enabled.
pub fn setup_debug_messenger(
    entry: &Entry,
    instance: &Instance,
) -> Result<(vk::DebugUtilsMessengerEXT, debug_utils::Instance), Box<dyn Error>> {
    let debug_utils_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            // vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE | // They aren't joking
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(vulkan_debug_callback));

    let debug_utils_loader = debug_utils::Instance::new(entry, instance);

    unsafe {
        let debug_callback =
            debug_utils_loader.create_debug_utils_messenger(&debug_utils_create_info, None)?;

        Ok((debug_callback, debug_utils_loader))
    }
}
