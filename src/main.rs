use ash::ext::debug_utils;
use ash::khr::surface;

use ash::vk::SurfaceKHR;
use ash::{vk, Device, Entry, Instance};
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::ffi::CStr;
use std::thread;
use std::time::Duration;
use std::{error::Error, result::Result};
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

        // Doesn't like win32_surface? Windows only? Idk
        // https://github.com/adrien-ben/vulkan-tutorial-rs/blob/85d247c990a2058daf576160e63480b6eae8ac18/src/util.rs#L4
        // let extension_names = vec![surface::NAME.as_ptr(), win32_surface::NAME.as_ptr()];

        // let extension_names = util::get_extension_names(None);

        debug::check_validation_layer_support(entry);

        let (_layer_names, layer_names_ptrs) = debug::get_layer_names_and_pointers();

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
        graphics.is_some() && present.is_some()
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

        let device_features = vk::PhysicalDeviceFeatures::default();

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&device_features);

        let device = unsafe {
            instance
                .create_device(device, &device_create_info, None)
                .expect("Failed to create lopgical device.")
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };

        (device, graphics_queue, present_queue)
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
