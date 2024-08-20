use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};
use ash::vk::SurfaceKHR;
use ash::{vk, Entry, Instance};
use core::fmt;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::{borrow::Cow, error::Error, os::raw::c_void, result::Result};
use winit::raw_window_handle::RawDisplayHandle;

//////////////// Constants ////////////////
// This doesn't exist in this version of Ash
// const REQUIRED_LAYERS: [&'static str; 1] = ["VK_LAYER_LUNARG_standard_validation"];
// But this does
pub const REQUIRED_LAYERS: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];
pub const REQUIRED_DEVICE_EXTENSIONS: [&'static CStr; 1] = [swapchain::NAME];

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;

//////////////// My Error (AppError) ////////////////
#[derive(Debug)]
struct AppError {
    details: String,
}

impl AppError {
    fn new(msg: &str) -> Self {
        AppError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for AppError {
    fn description(&self) -> &str {
        &self.details
    }
}

/// Check if the required validation set in `REQUIRED_LAYERS`
/// are supported by the Vulkan instance.
///
/// # Panics
///
/// Panic if at least one on the layer is not supported.
pub fn check_validation_layer_support(entry: &Entry) -> Result<(), Box<dyn Error>> {
    let mut missing_layers: Vec<&str> = Vec::new();

    let instance_layer_properties = unsafe {
        entry
            .enumerate_instance_layer_properties()?
            .iter()
            .map(|layer| {
                CStr::from_ptr(layer.layer_name.as_ptr())
                    .to_str()
                    .expect("Failed to get layer name pointer")
            })
            .collect::<Vec<_>>()
    };

    for required_layer in REQUIRED_LAYERS.iter() {
        log::info!("Searching for {:?}", required_layer);
        if !instance_layer_properties.contains(required_layer) {
            log::info!("Missing {:?}", required_layer);
            missing_layers.push(required_layer);
        } else {
            log::info!("Found {:?}", required_layer);
        }
    }

    if missing_layers.is_empty() {
        Ok(())
    } else {
        let message = format!("Missing Validation Layers:\n{}", missing_layers.join("\n"));
        Err(Box::new(AppError::new(&message)))
    }
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

/// Vulkan extensions required by this application.
pub fn get_extension_names(
    display_handle: Option<RawDisplayHandle>,
) -> Result<Vec<*const i8>, Box<dyn Error>> {
    let extension_names = match display_handle {
        Some(raw_display_handle) => {
            let mut extension_names =
                ash_window::enumerate_required_extensions(raw_display_handle)?.to_vec();
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

    Ok(extension_names)
}

/// Debug Messenger callback function.
/// This gets called as the validation layers get triggered.
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

/// Place to store some information about physical devices Vulkan discovers,
/// mostly to determine their suitability for what we are attempting to do.
#[derive(Default, Debug, Clone)]
pub struct DeviceDetails {
    pub name: String,
    pub graphics_queue_index: u32,
    pub present_queue_index: u32,
}

impl fmt::Display for DeviceDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "-Name: {} -Graphics: {} -Present: {}",
            self.name, self.graphics_queue_index, self.present_queue_index
        )
    }
}

// impl DeviceDetails {
//     fn name(mut self, name: &str) -> Self {
//         self.name = name.to_string();
//         self
//     }

//     fn graphics_queue_index(mut self, index: u32) -> Self {
//         self.graphics_queue_index = index;
//         self
//     }

//     fn present_queue_index(mut self, index: u32) -> Self {
//         self.present_queue_index = index;
//         self
//     }
// }

/// Discover Devices Capable of Running Vulkan
pub fn physical_devices(instance: &Instance) -> Result<Vec<vk::PhysicalDevice>, Box<dyn Error>> {
    let mut devices: Vec<vk::PhysicalDevice> = Vec::new();

    unsafe { instance.enumerate_physical_devices()? }
        .iter()
        .for_each(|device| {
            let device_name = unsafe {
                let device_properties = instance.get_physical_device_properties(*device);
                CStr::from_ptr(device_properties.device_name.as_ptr())
                    .to_str()
                    .expect("Could not convert pointer into string.")
            };
            log::debug!("Discovered Device: {:?}", device_name);

            devices.push(*device);
        });

    Ok(devices)
}

/// Determine if discovered devices support the required device extensions (e.g., swapchain).
pub fn devices_extension_support(
    instance: &Instance,
    devices: Vec<vk::PhysicalDevice>,
) -> Result<Vec<vk::PhysicalDevice>, Box<dyn Error>> {
    log::debug!("Check Device Extension Support.");
    let mut supported_devices: Vec<vk::PhysicalDevice> = Vec::new();

    for device in devices.iter() {
        let extension_props = unsafe {
            let extension_props = instance.enumerate_device_extension_properties(*device)?;
            extension_props
                .iter()
                .map(|property| CStr::from_ptr(property.extension_name.as_ptr()))
                .collect::<Vec<_>>()
        };

        // log::debug!("{:?}", extension_props);

        let extensions_support = REQUIRED_DEVICE_EXTENSIONS.iter().all(|name| {
            log::debug!("Checking device {:?} for support for {:?}", device, name);
            extension_props.contains(name)
        });

        if extensions_support {
            supported_devices.push(*device);
        }
    }

    Ok(supported_devices)
}

/// Determine if discovered devices have an adequate swapchain
pub fn devices_swapchain_adequate(
    surface: &surface::Instance,
    surface_khr: SurfaceKHR,
    devices: Vec<vk::PhysicalDevice>,
) -> Result<Vec<vk::PhysicalDevice>, Box<dyn Error>> {
    log::debug!("Check SwapChain Adequacy.");

    let mut supported_devices: Vec<vk::PhysicalDevice> = Vec::new();

    for device in devices.iter() {
        let formats = unsafe { surface.get_physical_device_surface_formats(*device, surface_khr)? };
        let present_modes =
            unsafe { surface.get_physical_device_surface_present_modes(*device, surface_khr)? };

        if formats.is_empty() || present_modes.is_empty() {
            log::debug!(
                "Device {:?} swapchain is inadequate (does not support format or present_modes).",
                device
            );
        } else {
            supported_devices.push(*device);
        }
    }

    Ok(supported_devices)
}

/// HashMap<vk::PhysicalDevice, DeviceDetails> is a lot to write a bunch of times.
type DeviceMap = HashMap<vk::PhysicalDevice, DeviceDetails>;

/// Filter physical devices based on if they support required queues (present and graphics).
/// This SHOULD take only some form of "suitable" device,
/// filtered by devices_swapchain_adequate and devices_extension_support.
pub fn devices_queue_family_support(
    instance: &Instance,
    surface: &surface::Instance,
    surface_khr: SurfaceKHR,
    devices: Vec<vk::PhysicalDevice>,
) -> Result<DeviceMap, Box<dyn Error>> {
    log::debug!("Find queue families.");

    let mut supported_devices: DeviceMap = HashMap::new();

    for device in devices.iter() {
        let props = unsafe { instance.get_physical_device_queue_family_properties(*device) };

        let device_name = unsafe {
            let device_properties = instance.get_physical_device_properties(*device);
            CStr::from_ptr(device_properties.device_name.as_ptr())
                .to_str()
                .expect("Could not convert pointer into string.")
        };

        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let mut graphics: Option<u32> = None;
            let mut present: Option<u32> = None;

            let index = index as u32;
            log::debug!("Property {}: {:?}", index, family);

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics = Some(index);
            }

            let present_support = unsafe {
                surface.get_physical_device_surface_support(*device, index, surface_khr)?
            };

            if present_support {
                present = Some(index);
            }

            if graphics.is_some() && present.is_some() {
                supported_devices.insert(
                    *device,
                    DeviceDetails {
                        name: device_name.to_string(),
                        graphics_queue_index: index,
                        present_queue_index: index,
                    },
                );
            } else {
                log::debug!(
                    "Device {:?} does not support graphics or presentation.",
                    device
                );
            }
        }
    }

    Ok(supported_devices)
}

/// Picks the first available physical device in the device map
/// Extend functionality (e.g. rank devices) later.
/// Also, devices_..._support functions must be run first, which isn't enforced (and needs to be).
pub fn pick_physical_device(
    devices: &DeviceMap,
) -> Result<(vk::PhysicalDevice, DeviceDetails), Box<dyn Error>> {
    for (device, details) in devices.iter() {
        return Ok((*device, details.clone()));
    }
    return Err(Box::new(AppError::new(
        "No supported physical devices to choose from!",
    )));
}

/// [ ] ToDo: Some meaningful Description.
pub struct SwapChainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupportDetails {
    pub fn new(
        device: vk::PhysicalDevice,
        surface: &surface::Instance,
        surface_khr: SurfaceKHR,
    ) -> Result<Self, Box<dyn Error>> {
        let capabilities =
            unsafe { surface.get_physical_device_surface_capabilities(device, surface_khr)? };

        let formats = unsafe { surface.get_physical_device_surface_formats(device, surface_khr)? };

        let present_modes =
            unsafe { surface.get_physical_device_surface_present_modes(device, surface_khr)? };

        Ok(Self {
            capabilities,
            formats,
            present_modes,
        })
    }

    pub fn choose_swapchain_surface_format(&self) -> vk::SurfaceFormatKHR {
        if self.formats.len() == 1 && self.formats[0].format == vk::Format::UNDEFINED {
            return vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };
        }

        *self
            .formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_UNORM
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(&self.formats[0])
    }

    pub fn choose_swapchain_surface_present_mode(&self) -> vk::PresentModeKHR {
        if self.present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else if self.present_modes.contains(&vk::PresentModeKHR::FIFO) {
            vk::PresentModeKHR::FIFO
        } else {
            vk::PresentModeKHR::IMMEDIATE
        }
    }

    pub fn choose_swapchain_extent(&self) -> vk::Extent2D {
        if self.capabilities.current_extent.width != std::u32::MAX {
            return self.capabilities.current_extent;
        }

        let min = self.capabilities.min_image_extent;
        let max = self.capabilities.max_image_extent;
        let width = WIDTH.min(max.width).max(min.width);
        let height = HEIGHT.min(max.height).max(min.height);

        vk::Extent2D { width, height }
    }
}
