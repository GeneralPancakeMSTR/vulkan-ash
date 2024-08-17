use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};

use ash::vk::SurfaceKHR;
use ash::{vk, Device, Entry, Instance};
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::ffi::CStr;
use std::thread;
use std::time::Duration;
use std::{error::Error, result::Result};
use winit::dpi::PhysicalSize;
use winit::platform::x11::WindowAttributesExtX11;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use winit::platform::startup_notify::{
    self, EventLoopExtStartupNotify, WindowAttributesExtStartupNotify,
};
use winit::{
    application::ApplicationHandler, event_loop::ActiveEventLoop, event_loop::EventLoop,
    window::Window, window::WindowId,
};

use std::sync::Arc;
use std::sync::Mutex;

mod debug;
mod util;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// Need to use underscores: "If the binary name contains hyphens, you will need to replace them with underscores:"
// RUST_LOG=vulkan_ash_tutorial=debug cargo run
// Not helping: RUST_LOG=vulkan_ash_tutorial=debug cargo run --target x86_64-unknown-linux-gnu

// Main tutorial this is following: https://github.com/adrien-ben/vulkan-tutorial-rs
// Useful (up-to-date) example: https://github.com/ash-rs/ash/blob/master/ash-examples/src/lib.rs#L601
// Another resource, but also using older API: https://hoj-senna.github.io/ashen-aetna/text/003_Validation_layers.html
// Vulkan Documentation could be helpful, though it's for the C++: https://docs.vulkan.org/spec/latest/index.html
// Also Note: https://github.com/ash-rs/ash/releases/tag/0.38.0
// winit also changed: https://github.com/rust-windowing/winit/releases/tag/v0.30.0
// Also need an example https://github.com/rust-windowing/winit/blob/master/examples/window.rs

#[derive(Debug)]
enum EventLoopProxyEvent {
    RequestWindowHandle,
}

struct Application {
    windows: HashMap<WindowId, Arc<Window>>,
    shared_window: Arc<Mutex<Option<Arc<Window>>>>,
}

impl ApplicationHandler<EventLoopProxyEvent> for Application {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let verbose = false;
        if verbose {
            log::debug!("Window Event {:?}", event_loop);
            log::debug!("Window Event {:?}", event);
        }

        let _window = match self.windows.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Resumed {:?}", event_loop);

        match self.windows.iter().next() {
            Some(window) => log::debug!("{:?}", window),
            None => {
                log::debug!("create window");
                let (window_id, window) = Self::create_window(event_loop).unwrap();
                self.windows.insert(window_id, window);
            }
        };
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: EventLoopProxyEvent) {
        match event {
            EventLoopProxyEvent::RequestWindowHandle => match self.windows.iter().next() {
                Some(window) => {
                    let mut shared_window = self.shared_window.lock().unwrap();
                    *shared_window = Some(Arc::clone(&window.1))
                }
                None => {
                    log::debug!("No Window Created.")
                }
            },
        }
    }
}

impl Application {
    fn create_window(
        event_loop: &ActiveEventLoop,
    ) -> Result<(WindowId, Arc<Window>), Box<dyn Error>> {
        let mut window_attributes = Window::default_attributes().with_title("Vulkan Ash Tutorial");
        window_attributes = window_attributes.with_base_size(PhysicalSize {
            width: WIDTH,
            height: HEIGHT,
        });

        // If x11/wayland
        if let Some(token) = event_loop.read_token_from_env() {
            startup_notify::reset_activation_token_env();
            log::info!("Using token {:?} to activate a window", token);
            window_attributes = window_attributes.with_activation_token(token);
        }

        let window = event_loop.create_window(window_attributes)?;

        Ok((window.id(), Arc::new(window)))
    }
}

//////////////// VulkanProvider ////////////////
// I'm not sure about this approach
// But maybe?
// Also, maybe later ...

// struct VulkanProvider;

// impl VulkanProvider {
//     fn entry() -> Result<Entry, Box<dyn Error>> {
//         log::debug!("Create Vulkan Entry");

//         let entry = unsafe { Entry::load()? };
//         Ok(entry)
//     }
// }

////////////////   ////////////////
struct VulkanApp {
    _entry: Entry,
    instance: Instance,
    debug_utils_loader: debug_utils::Instance,
    debug_callback: vk::DebugUtilsMessengerEXT,
    surface: surface::Instance,
    surface_khr: SurfaceKHR,
    device: Device,
    _physical_device: vk::PhysicalDevice,
    _graphics_queue: vk::Queue,
    _present_queue: vk::Queue,
    swapchain: swapchain::Device,
    swapchain_khr: vk::SwapchainKHR,
    _images: Vec<vk::Image>,
    _swapchain_image_format: vk::Format,
    _swapchain_extent: vk::Extent2D,
    swapchain_image_views: Vec<vk::ImageView>,
}

impl VulkanApp {
    fn new(window: &Arc<Window>) -> Result<Self, Box<dyn Error>> {
        log::debug!("Creating Application");
        let entry = unsafe { Entry::load()? };

        let instance = Self::create_instance(&entry, window)?;

        let (debug_callback, debug_utils_loader) = debug::setup_debug_messenger(&entry, &instance)?;

        let (surface_khr, surface_loader) = Self::create_surface(&entry, &instance, window)?;

        let physical_device = Self::pick_physical_device(&instance, &surface_loader, surface_khr);

        let (device, graphics_queue, present_queue) =
            Self::create_logical_device_with_graphics_queue(
                &instance,
                &surface_loader,
                surface_khr,
                physical_device,
            );

        let (swapchain_loader, swapchain_khr, format, extent, images) =
            Self::create_swapchain_and_images(
                &instance,
                physical_device,
                &device,
                &surface_loader,
                surface_khr,
            );

        let swapchain_image_views = Self::create_swapchain_image_views(&device, &images, format);

        Ok(Self {
            _entry: entry,
            instance: instance,
            debug_utils_loader: debug_utils_loader,
            debug_callback: debug_callback,
            surface: surface_loader,
            surface_khr: surface_khr,
            device: device,
            _physical_device: physical_device,
            _graphics_queue: graphics_queue,
            _present_queue: present_queue,
            swapchain: swapchain_loader,
            swapchain_khr: swapchain_khr,
            _images: images,
            _swapchain_image_format: format,
            _swapchain_extent: extent,
            swapchain_image_views: swapchain_image_views,
        })
    }

    fn run(&mut self) {
        log::info!("Running application");
    }

    fn create_instance(
        entry: &Entry,
        // event_loop: Option<&EventLoop<()>>,
        window: &Arc<Window>,
    ) -> Result<Instance, Box<dyn Error>> {
        // This is the same as "..?"
        // let application_name = match CString::new("Tutorial Vulkan Application") {
        //     Ok(value) => value,
        //     Err(e) => return Result::Err(Box::new(e)),
        // };

        let app_info = unsafe {
            vk::ApplicationInfo::default()
                .api_version(vk::make_api_version(0, 1, 0, 0))
                .application_name(CStr::from_bytes_with_nul_unchecked(
                    b"Tutorial Vulkan Application\0",
                ))
                .engine_name(CStr::from_bytes_with_nul_unchecked(b"No Engine\0"))
                .engine_version(ash::vk::make_api_version(0, 1, 0, 0))
        };

        debug::check_validation_layer_support(entry);

        let (_layer_names, layer_names_ptrs) = debug::get_layer_names_and_pointers();

        // Doesn't like win32_surface? Windows only? Idk
        // https://github.com/adrien-ben/vulkan-tutorial-rs/blob/85d247c990a2058daf576160e63480b6eae8ac18/src/util.rs#L4
        // let extension_names = vec![surface::NAME.as_ptr(), win32_surface::NAME.as_ptr()];

        let extension_names = util::get_extension_names(Some(window.display_handle()?.as_raw()));

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&layer_names_ptrs)
            .enabled_extension_names(&extension_names)
            .flags(vk::InstanceCreateFlags::default());

        unsafe { Ok(entry.create_instance(&instance_create_info, None)?) }
    }

    fn pick_physical_device(
        instance: &Instance,
        surface: &surface::Instance,
        surface_khr: SurfaceKHR,
    ) -> vk::PhysicalDevice {
        let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        let device = devices
            .into_iter()
            .find(|device| Self::is_device_suitable(instance, surface, surface_khr, *device))
            .expect("No suitable physical device.");

        let props = unsafe { instance.get_physical_device_properties(device) };
        log::debug!("Selected physical device: {:?}", unsafe {
            CStr::from_ptr(props.device_name.as_ptr())
        });

        device
    }

    fn is_device_suitable(
        instance: &Instance,
        surface: &surface::Instance,
        surface_khr: SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> bool {
        let (graphics, present) = Self::find_queue_families(instance, surface, surface_khr, device);
        let extension_support = Self::check_device_extension_support(instance, device);

        let is_swapchain_adequate = {
            let details = SwapChainSupportDetails::new(device, surface, surface_khr);
            !details.formats.is_empty() && !details.present_modes.is_empty()
        };

        graphics.is_some() && present.is_some() && extension_support && is_swapchain_adequate
    }

    fn check_device_extension_support(instance: &Instance, device: vk::PhysicalDevice) -> bool {
        let required_extensions = Self::get_required_device_extensions();

        let extension_props = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
        };

        for required_extension in required_extensions.iter() {
            let found = extension_props.iter().any(|ext| {
                let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                required_extension == &name
            });

            if !found {
                return false;
            }
        }

        true
    }

    fn get_required_device_extensions() -> [&'static CStr; 1] {
        [swapchain::NAME]
    }

    fn find_queue_families(
        instance: &Instance,
        surface: &surface::Instance,
        surface_khr: SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> (Option<u32>, Option<u32>) {
        log::debug!("Find queue families.");
        let mut graphics: Option<u32> = None;
        let mut present: Option<u32> = None;

        let props = unsafe { instance.get_physical_device_queue_family_properties(device) };

        for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
            let index = index as u32;

            log::debug!("Property Index {}", index);
            log::debug!("Property value {:?}", family);

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
                graphics = Some(index);
            }

            let present_support =
                unsafe { surface.get_physical_device_surface_support(device, index, surface_khr) };

            match present_support {
                Ok(present_support) => {
                    if present_support && present.is_none() {
                        present = Some(index);
                    }
                }
                Err(err) => {
                    log::error!("Error getting physical device support: {}", err)
                }
            }
        }

        (graphics, present)
    }

    fn create_logical_device_with_graphics_queue(
        instance: &Instance,
        surface: &surface::Instance,
        surface_khr: SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> (Device, vk::Queue, vk::Queue) {
        let (graphics, present) = Self::find_queue_families(instance, surface, surface_khr, device);

        let graphics_family_index = graphics.unwrap();
        let present_family_index = present.unwrap();

        let queue_priorities = [1.0f32];

        let mut queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = vec![];

        let mut queue_indices = vec![graphics_family_index, present_family_index];
        queue_indices.dedup();

        for index in queue_indices.iter() {
            let queue_create_info: vk::DeviceQueueCreateInfo = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(*index)
                .queue_priorities(&queue_priorities);

            queue_create_infos.push(queue_create_info);
        }

        let device_extensions = Self::get_required_device_extensions();
        let device_extension_ptrs = device_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let device_features = vk::PhysicalDeviceFeatures::default();

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&device_features)
            .enabled_extension_names(&device_extension_ptrs);

        let device = unsafe {
            instance
                .create_device(device, &device_create_info, None)
                .expect("Failed to create lopgical device.")
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };

        (device, graphics_queue, present_queue)
    }

    fn create_swapchain_and_images(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        device: &Device,
        surface: &surface::Instance,
        surface_khr: SurfaceKHR,
    ) -> (
        swapchain::Device,
        vk::SwapchainKHR,
        vk::Format,
        vk::Extent2D,
        Vec<vk::Image>,
    ) {
        let details = SwapChainSupportDetails::new(physical_device, surface, surface_khr);
        let format = Self::choose_swapchain_surface_format(&details.formats);
        let present_mode = Self::choose_swapchain_surface_present_mode(&details.present_modes);
        let extent = Self::choose_swapchain_extent(details.capabilities);

        let image_count = {
            let max = details.capabilities.max_image_count;
            let mut preferred = details.capabilities.min_image_count + 1;
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

        let (graphics, present) =
            Self::find_queue_families(instance, surface, surface_khr, physical_device);

        let families_indices = [graphics.unwrap(), present.unwrap()];

        let swap_chain_create_info = {
            let mut swap_chain_create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface_khr)
                .min_image_count(image_count)
                .image_format(format.format)
                .image_color_space(format.color_space)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

            swap_chain_create_info = match (graphics, present) {
                (Some(graphics), Some(present)) if graphics != present => swap_chain_create_info
                    .image_sharing_mode(vk::SharingMode::CONCURRENT)
                    .queue_family_indices(&families_indices),
                (Some(_), Some(_)) => {
                    swap_chain_create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                }
                _ => panic!(),
            };

            swap_chain_create_info
                .pre_transform(details.capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
        };

        let swapchain_loader = swapchain::Device::new(instance, device);

        let swapchain_khr = unsafe {
            swapchain_loader
                .create_swapchain(&swap_chain_create_info, None)
                .unwrap()
        };

        let images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain_khr)
                .unwrap()
        };

        (
            swapchain_loader,
            swapchain_khr,
            format.format,
            extent,
            images,
        )
    }

    fn choose_swapchain_surface_format(
        available_formats: &[vk::SurfaceFormatKHR],
    ) -> vk::SurfaceFormatKHR {
        if available_formats.len() == 1 && available_formats[0].format == vk::Format::UNDEFINED {
            return vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };
        }

        *available_formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_UNORM
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(&available_formats[0])
    }

    fn choose_swapchain_surface_present_mode(
        available_present_modes: &[vk::PresentModeKHR],
    ) -> vk::PresentModeKHR {
        if available_present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else if available_present_modes.contains(&vk::PresentModeKHR::FIFO) {
            return vk::PresentModeKHR::FIFO;
        } else {
            vk::PresentModeKHR::IMMEDIATE
        }
    }

    fn choose_swapchain_extent(capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if capabilities.current_extent.width != std::u32::MAX {
            return capabilities.current_extent;
        }

        let min = capabilities.min_image_extent;
        let max = capabilities.max_image_extent;
        let width = WIDTH.min(max.width).max(min.width);
        let height = HEIGHT.min(max.height).max(min.height);

        vk::Extent2D { width, height }
    }

    fn create_swapchain_image_views(
        device: &Device,
        swapchain_images: &[vk::Image],
        swapchain_format: vk::Format,
    ) -> Vec<vk::ImageView> {
        swapchain_images
            .into_iter()
            .map(|image| {
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

                unsafe {
                    device
                        .create_image_view(&image_view_create_info, None)
                        .unwrap()
                }
            })
            .collect::<Vec<_>>()
    }

    fn create_surface(
        entry: &Entry,
        instance: &Instance,
        window: &Arc<Window>,
    ) -> Result<(SurfaceKHR, surface::Instance), Box<dyn Error>> {
        log::debug!("Create Surface.");

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
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        log::debug!("Dropping application.");
        unsafe {
            self.swapchain_image_views
                .iter()
                .for_each(|v| self.device.destroy_image_view(*v, None));
            self.swapchain.destroy_swapchain(self.swapchain_khr, None);
            self.device.destroy_device(None);
            self.surface.destroy_surface(self.surface_khr, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}

struct SwapChainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupportDetails {
    fn new(
        device: vk::PhysicalDevice,
        surface: &surface::Instance,
        surface_khr: SurfaceKHR,
    ) -> Self {
        let capabilities = unsafe {
            surface
                .get_physical_device_surface_capabilities(device, surface_khr)
                .unwrap()
        };

        let formats = unsafe {
            surface
                .get_physical_device_surface_formats(device, surface_khr)
                .unwrap()
        };

        let present_modes = unsafe {
            surface
                .get_physical_device_surface_present_modes(device, surface_khr)
                .unwrap()
        };

        Self {
            capabilities,
            formats,
            present_modes,
        }
    }
}

fn main() {
    env_logger::init();
    // They don't want you to run event_loop outside the main thread.
    let event_loop = EventLoop::<EventLoopProxyEvent>::with_user_event()
        .build()
        .unwrap();

    let shared_window: Arc<Mutex<Option<Arc<Window>>>> = Arc::new(Mutex::new(None));

    let window_shared_to_graphics_thread = Arc::clone(&shared_window);

    let event_loop_proxy = event_loop.create_proxy();

    let _graphics_thread = thread::spawn(move || {
        let mut vulkan_app: Option<VulkanApp> = None;

        loop {
            log::debug!("Check for window.");
            let shared_window = window_shared_to_graphics_thread.lock().unwrap();
            match (*shared_window).borrow() {
                Some(window) => {
                    log::debug!("Found window {:?}", window);
                    match vulkan_app {
                        Some(_) => break,
                        None => {
                            log::debug!("Create Vulkan App.");
                            vulkan_app = match VulkanApp::new(window) {
                                Ok(app) => Some(app),
                                Err(err) => {
                                    log::error!(
                                        "Encountered some error trying to create Vulkan App: {}",
                                        err
                                    );
                                    None
                                }
                            }
                        }
                    }
                }
                None => {
                    log::debug!("Request Window.");
                    let _ = event_loop_proxy.send_event(EventLoopProxyEvent::RequestWindowHandle);
                }
            }

            thread::sleep(Duration::from_secs(1));
        }

        log::debug!("Run Vulkan App.");

        match vulkan_app {
            Some(ref mut app) => {
                app.run();
                // Deliberately drop Vulkan App (to test Drop implementation)
                // loop {
                //     log::debug!("Vulkan App Running");
                //     thread::sleep(Duration::from_secs(1));
                // }
            }
            None => log::error!("Vulkan App not missing?"),
        }
    });

    let mut app = Application {
        windows: Default::default(),
        shared_window: shared_window,
    };

    event_loop.run_app(app.borrow_mut()).unwrap();
}
