use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk::SurfaceKHR,
    Device,
};
use std::{error::Error, ffi::CStr, sync::Arc};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use ash::{vk, Entry, Instance};

use crate::util::{self, DeviceDetails, SwapChainSupportDetails};

//////////////// Create Vulkan Things Helper Functions ////////////////

/// Create Vulkan instance from an entry point, layers (e.g. validation), and extensions.
pub fn instance(
    entry: &Entry,
    layer_names_ptrs: Vec<*const i8>,
    extension_names: Vec<*const i8>,
) -> Result<Instance, Box<dyn Error>> {
    // This is the same as "..?"
    // let application_name = match CString::new("Tutorial Vulkan Application") {
    //     Ok(value) => value,
    //     Err(e) => return Result::Err(Box::new(e)),
    // };

    // debug::check_validation_layer_support(entry);

    // let (_layer_names, layer_names_ptrs) = debug::get_layer_names_and_pointers();

    // Doesn't like win32_surface? Windows only? Idk
    // https://github.com/adrien-ben/vulkan-tutorial-rs/blob/85d247c990a2058daf576160e63480b6eae8ac18/src/util.rs#L4
    // let extension_names = vec![surface::NAME.as_ptr(), win32_surface::NAME.as_ptr()];

    // let extension_names = util::get_extension_names(Some(window.display_handle()?.as_raw()));

    let app_info = unsafe {
        vk::ApplicationInfo::default()
            .api_version(vk::make_api_version(0, 1, 0, 0))
            .application_name(CStr::from_bytes_with_nul_unchecked(
                b"Tutorial Vulkan Application\0",
            ))
            .engine_name(CStr::from_bytes_with_nul_unchecked(b"No Engine\0"))
            .engine_version(ash::vk::make_api_version(0, 1, 0, 0))
    };

    let instance_create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&layer_names_ptrs)
        .enabled_extension_names(&extension_names)
        .flags(vk::InstanceCreateFlags::default());

    unsafe { Ok(entry.create_instance(&instance_create_info, None)?) }
}

/// Setup the debug message if validation layers are enabled.
pub fn debug_messenger(
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
        .pfn_user_callback(Some(util::vulkan_debug_callback));

    let debug_utils_loader = debug_utils::Instance::new(entry, instance);
    unsafe {
        let debug_callback =
            debug_utils_loader.create_debug_utils_messenger(&debug_utils_create_info, None)?;

        Ok((debug_callback, debug_utils_loader))
    }
}

/// Create a Vulkan surface from the entry, instance, and a (shared) winit window.
pub fn surface(
    entry: &Entry,
    instance: &Instance,
    window: &Arc<Window>,
) -> Result<(SurfaceKHR, surface::Instance), Box<dyn Error>> {
    let surface_khr = unsafe {
        ash_window::create_surface(
            entry,
            instance,
            window.display_handle()?.as_raw(),
            window.window_handle()?.as_raw(),
            None,
        )?
    };

    let surface_loader = surface::Instance::new(entry, instance);

    Ok((surface_khr, surface_loader))
}

/// Create the Vulkan Device with a graphics queue.
pub fn logical_device_with_graphics_queue(
    instance: &Instance,
    device: vk::PhysicalDevice,
    device_details: &DeviceDetails,
) -> Result<(Device, vk::Queue, vk::Queue), Box<dyn Error>> {
    let (graphics_family_index, present_family_index) = (
        device_details.graphics_queue_index,
        device_details.present_queue_index,
    );

    let queue_priorities = [1.0f32];

    let mut queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = vec![];

    let mut queue_indices = vec![graphics_family_index, present_family_index];
    queue_indices.dedup();

    for index in queue_indices.iter() {
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(*index)
            .queue_priorities(&queue_priorities);

        queue_create_infos.push(queue_create_info);
    }

    let device_extensions = util::REQUIRED_DEVICE_EXTENSIONS;
    let device_extension_ptrs = device_extensions
        .iter()
        .map(|ext| ext.as_ptr())
        .collect::<Vec<_>>();

    let device_features = vk::PhysicalDeviceFeatures::default();

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_features(&device_features)
        .enabled_extension_names(&device_extension_ptrs);

    let device = unsafe { instance.create_device(device, &device_create_info, None)? };

    let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
    let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };

    Ok((device, graphics_queue, present_queue))
}

/// Construct swapchain and image views.
pub fn swapchain_and_images(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    device_details: &DeviceDetails,
    device: &Device,
    surface: &surface::Instance,
    surface_khr: SurfaceKHR,
) -> Result<
    (
        swapchain::Device,
        vk::SwapchainKHR,
        vk::Format,
        vk::Extent2D,
        Vec<vk::Image>,
    ),
    Box<dyn Error>,
> {
    let swapchain_support_details =
        SwapChainSupportDetails::new(physical_device, surface, surface_khr)?;

    let format = swapchain_support_details.choose_swapchain_surface_format();
    let present_mode = swapchain_support_details.choose_swapchain_surface_present_mode();
    let extent = swapchain_support_details.choose_swapchain_extent();

    let image_count = {
        let max = swapchain_support_details.capabilities.max_image_count;
        let mut preferred = swapchain_support_details.capabilities.min_image_count + 1;
        if max > 0 && preferred > max {
            preferred = max;
        }
        preferred
    };

    log::debug!("Creating swapchain");
    log::debug!("   - Format: {:?}", format.format);
    log::debug!("   - ColorSpace: {:?}", format.color_space);
    log::debug!("   - PresentMode: {:?}", present_mode);
    log::debug!("   - Extent: {:?}", extent);
    log::debug!("   - ImageCount: {:?}", image_count);

    let families_indices = [
        device_details.graphics_queue_index,
        device_details.present_queue_index,
    ];

    let swapchain_create_info = {
        let mut swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface_khr)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

        swapchain_create_info = match (
            device_details.graphics_queue_index,
            device_details.present_queue_index,
        ) {
            (graphics, present) if graphics != present => swapchain_create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&families_indices),
            (_, _) => swapchain_create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE),
        };

        swapchain_create_info
            .pre_transform(swapchain_support_details.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
    };

    let swapchain_loader = swapchain::Device::new(instance, device);

    let swapchain_khr = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

    let images = unsafe { swapchain_loader.get_swapchain_images(swapchain_khr)? };

    Ok((
        swapchain_loader,
        swapchain_khr,
        format.format,
        extent,
        images,
    ))
}

/// Create image views from swapchain images.
pub fn swapchain_image_views(
    device: &Device,
    swapchain_images: &[vk::Image],
    swapchain_format: vk::Format,
) -> Result<Vec<vk::ImageView>, Box<dyn Error>> {
    let mut image_views: Vec<vk::ImageView> = Vec::new();
    for image in swapchain_images.into_iter() {
        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .image(*image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(swapchain_format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe { device.create_image_view(&image_view_create_info, None)? };
        image_views.push(image_view);
    }

    Ok(image_views)
}
