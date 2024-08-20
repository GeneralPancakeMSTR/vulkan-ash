use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};

use ash::vk::SurfaceKHR;
use ash::{vk, Device, Entry, Instance};
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::{error::Error, result::Result};
use winit::dpi::PhysicalSize;
use winit::platform::x11::WindowAttributesExtX11;
use winit::raw_window_handle::HasDisplayHandle;

use winit::platform::startup_notify::{
    self, EventLoopExtStartupNotify, WindowAttributesExtStartupNotify,
};
use winit::{
    application::ApplicationHandler, event_loop::ActiveEventLoop, event_loop::EventLoop,
    window::Window, window::WindowId,
};

use std::sync::Arc;
use std::sync::Mutex;

// mod debug;
mod util;
mod vulkan_create;

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
            width: util::WIDTH,
            height: util::HEIGHT,
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

        //////////////// Refactor ////////////////
        util::check_validation_layer_support(&entry)?;

        let (_layer_names, layer_names_ptrs) = util::get_layer_names_and_pointers();

        let extension_names = util::get_extension_names(Some(window.display_handle()?.as_raw()))?;

        let instance = vulkan_create::instance(&entry, layer_names_ptrs, extension_names)?;

        let (debug_callback, debug_utils_loader) =
            vulkan_create::debug_messenger(&entry, &instance)?;

        let (surface_khr, surface_loader) = vulkan_create::surface(&entry, &instance, window)?;

        let mut devices = util::physical_devices(&instance)?;

        // This needs a little work, nothing enforces you to run these two commands.
        devices = util::devices_extension_support(&instance, devices)?;
        devices = util::devices_swapchain_adequate(&surface_loader, surface_khr, devices)?;

        // This should only be able to take in some form of suitable device,
        // filtered by util::devices_extension_support and util::devices_swapchain_adequate.
        let physical_devices =
            util::devices_queue_family_support(&instance, &surface_loader, surface_khr, devices)?;

        log::debug!("Found Physical Devices: {:?}", physical_devices);

        let (physical_device, device_details) = util::pick_physical_device(&physical_devices)?;

        log::debug!(
            "Selected Physical Device {:?} ({:?})",
            physical_device,
            device_details
        );

        let (device, graphics_queue, present_queue) =
            vulkan_create::logical_device_with_graphics_queue(
                &instance,
                physical_device,
                &device_details,
            )?;

        let (swapchain_loader, swapchain_khr, format, extent, images) =
            vulkan_create::swapchain_and_images(
                &instance,
                physical_device,
                &device_details,
                &device,
                &surface_loader,
                surface_khr,
            )?;

        let swapchain_image_views = vulkan_create::swapchain_image_views(&device, &images, format)?;

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
