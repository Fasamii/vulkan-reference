use ash::{ext, khr, vk};
use std::{
    ffi::{CStr, c_char},
    ptr::fn_addr_eq,
};
use winit::platform::wayland::WindowAttributesExtWayland;

const APP_NAME: &CStr = c"VULKAN-REFERENCE";
const ENGINE_NAME: &CStr = c"NO ENGINE";
const INSTANCE_LAYERS: &[*const c_char] = &[
    c"VK_LAYER_KHRONOS_validation".as_ptr() as *const c_char,
    // c"VK_LAYER_LUNARG_monitor".as_ptr() as *const c_char,
    // c"VK_LAYER_LUNARG_api_dump".as_ptr() as *const c_char,
    khr::swapchain::NAME.as_ptr() as *const c_char,
];
const INSTANCE_EXTENSIONS: &[*const c_char] = &[ext::debug_utils::NAME.as_ptr() as *const c_char];

#[allow(unused)]
struct VulkanBase {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    graphics_queue_idx: usize,
    device: ash::Device,
    queue: vk::Queue,
}

impl Drop for VulkanBase {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl VulkanBase {
    pub fn new(surface: vk::SurfaceKHR) -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { ash::Entry::load()? };

        // Checking if all layers are available
        let available_layers = unsafe { entry.enumerate_instance_layer_properties()? };
        for &layer_ptr in INSTANCE_LAYERS {
            let layer_name = unsafe { CStr::from_ptr(layer_ptr) };
            let found = available_layers.iter().any(|prop| {
                let prop_name = unsafe { CStr::from_ptr(prop.layer_name.as_ptr()) };
                prop_name == layer_name
            });

            if !found {
                panic!("Layer {layer_name:?} not supported");
            }
        }

        let application_info = vk::ApplicationInfo::default()
            .application_name(APP_NAME)
            .engine_name(ENGINE_NAME)
            .api_version(vk::API_VERSION_1_3);

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_layer_names(INSTANCE_LAYERS)
            .enabled_extension_names(INSTANCE_EXTENSIONS);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        let physical_devices = unsafe { instance.enumerate_physical_devices()? };

        // Select first graphics device
        let physical_device = physical_devices
            .into_iter()
            .nth(0)
            .ok_or("No vulkan physical devices found")?;

        // Find graphics queue
        let graphics_queue_idx =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) }
                .iter()
                .enumerate()
                .find_map(|(idx, prop)| {
                    let supports_graphics = prop.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                    if supports_graphics { Some(idx) } else { None }
                })
                .ok_or("No GRAPHICS queue on device")?;

        let device_queue_create_info = [vk::DeviceQueueCreateInfo::default()
            .queue_priorities(&[1f32])
            .queue_family_index(graphics_queue_idx as u32)];

        let device_create_info =
            vk::DeviceCreateInfo::default().queue_create_infos(&device_queue_create_info);

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        let queue = unsafe { device.get_device_queue(graphics_queue_idx as u32, 0) };

        Ok(Self {
            entry,
            instance,
            physical_device,
            graphics_queue_idx,
            device,
            queue,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut handle = Handle { application: None };
    event_loop.run_app(&mut handle)?;
    Ok(())
}

struct App {
    window: winit::window::Window,
    vulkan: VulkanBase,
}

struct Handle {
    application: Option<App>,
}

impl winit::application::ApplicationHandler for Handle {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.application.is_none() {
            let window = event_loop
                .create_window(winit::window::WindowAttributes::default())
                .expect("Failed to create window");

            let vulkan = VulkanBase::new(todo!()).expect("Failed to create vulkan context");

            let app = App { window, vulkan };

            self.application = Some(app);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => println!("Close requested"),
            winit::event::WindowEvent::RedrawRequested => println!("Redraw requeste"),
            _ => (),
        }
    }
}
