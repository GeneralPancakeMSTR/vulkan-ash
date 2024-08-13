use ash::ext::debug_utils;
use ash::khr::surface;
use winit::raw_window_handle::RawDisplayHandle;

pub fn get_extension_names(display_handle: Option<RawDisplayHandle>) -> Vec<*const i8> {
    let extension_names = match display_handle {
        Some(raw_display_handle) => {
            let mut extension_names = ash_window::enumerate_required_extensions(raw_display_handle)
                .unwrap()
                .to_vec();
            extension_names.push(surface::NAME.as_ptr());
            extension_names.push(debug_utils::NAME.as_ptr());
            extension_names
        }
        None => vec![
            surface::NAME.as_ptr(),
            // win32_surface::NAME.as_ptr(), // Does not work (on linux?)
            debug_utils::NAME.as_ptr(),
        ],
    };

    extension_names
}
