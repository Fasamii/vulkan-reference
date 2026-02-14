use ash::{ext, khr, vk};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::{CStr, c_char};

const APP_NAME: &CStr = c"VULKAN-SANDBOX";
const ENGINE_NAME: &CStr = c"NO ENGINE";
const INSTANCE_LAYERS: &[*const c_char] = &[
    c"VK_LAYER_KHRONOS_validation".as_ptr() as *const c_char,
    // c"VK_LAYER_LUNARG_monitor".as_ptr() as *const c_char,
    // c"VK_LAYER_LUNARG_api_dump".as_ptr() as *const c_char,
];
const INSTANCE_EXTENSIONS: &[*const c_char] = &[];
const DEVICE_EXTENSIONS: &[*const c_char] = &[
    khr::swapchain::NAME.as_ptr() as *const c_char, // For swapchain support
];

pub struct Context {
    instance: Instance,
    surface: Surface,
    device: Device,
    swapchain: Swapchain,
}

impl Context {
    pub fn new(window: &winit::window::Window) -> Self {
        let instance = Instance::new(window).expect("Instance Error");
        let surface = Surface::new(&instance, window).expect("Surface Error");
        let device = Device::new(&instance, &surface).expect("Device Error");
        let swapchain =
            Swapchain::new(&instance, &device, &surface, window, None).expect("Swapchain Error");

        Self {
            instance,
            surface,
            device,
            swapchain,
        }
    }

    pub fn recreate_swapchain(
        &mut self,
        window: &winit::window::Window,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let new_swapchain = Swapchain::new(
            &self.instance,
            &self.device,
            &self.surface,
            window,
            Some(self.swapchain.swapchain),
        )
        .expect("Swapchain Recreation Error");

        self.swapchain = new_swapchain;

        Ok(())
    }
}

pub struct Instance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

impl Instance {
    pub fn new(window: &winit::window::Window) -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { ash::Entry::load()? };

        // Create extensions vector
        let mut extension_names: Vec<*const i8> = Vec::from(INSTANCE_EXTENSIONS);

        use raw_window_handle::HasDisplayHandle;
        extension_names.append(
            &mut ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?
                .to_vec(),
        );

        // Add platform-specific surface extensions
        #[cfg(target_os = "windows")]
        extension_names.push(khr::win32_surface::NAME.as_ptr());
        #[cfg(target_os = "linux")]
        extension_names.push(khr::wayland_surface::NAME.as_ptr());
        #[cfg(target_os = "linux")]
        extension_names.push(khr::xlib_surface::NAME.as_ptr());
        #[cfg(target_os = "macos")]
        extension_names.push(ext::metal_surface::NAME.as_ptr());
        #[cfg(target_os = "macos")]
        extension_names.push(khr::portability_enumeration::NAME.as_ptr());

        // Verify layers are available
        let available_layers = unsafe { entry.enumerate_instance_layer_properties()? };
        for &layer_ptr in INSTANCE_LAYERS {
            let layer_name = unsafe { CStr::from_ptr(layer_ptr) };
            let found = available_layers.iter().any(|prop| {
                let prop_name = unsafe { CStr::from_ptr(prop.layer_name.as_ptr()) };
                prop_name == layer_name
            });
            if !found {
                panic!("Warning: Layer {layer_name:?} not available");
            }
        }

        let app_info = vk::ApplicationInfo::default()
            .application_name(APP_NAME)
            .engine_name(ENGINE_NAME)
            .api_version(vk::API_VERSION_1_3);

        let mut create_flags = vk::InstanceCreateFlags::default();
        #[cfg(target_os = "macos")]
        {
            create_flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
        }

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(INSTANCE_LAYERS)
            .enabled_extension_names(&extension_names)
            .flags(create_flags);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        Ok(Self { entry, instance })
    }
}

pub struct Surface {
    surface: vk::SurfaceKHR,
    loader: khr::surface::Instance,

    window_handle: raw_window_handle::RawWindowHandle,
    display_handle: raw_window_handle::RawDisplayHandle,
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}

impl Surface {
    fn new(
        instance: &Instance,
        window: &winit::window::Window,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window_handle = window.window_handle()?.as_raw();
        let display_handle = window.display_handle()?.as_raw();

        let loader = khr::surface::Instance::new(&instance.entry, &instance.instance);
        let surface = unsafe {
            ash_window::create_surface(
                &instance.entry,
                &instance.instance,
                display_handle,
                window_handle,
                None,
            )
        }?;

        Ok(Self {
            surface,
            loader,
            window_handle,
            display_handle,
        })
    }
}

pub struct Device {
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,

    pub graphics_queue_family_idx: u32,
    pub graphics_queue: vk::Queue,

    pub present_queue_family_idx: u32,
    pub present_queue: vk::Queue,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
        }
    }
}

impl Device {
    pub fn new(instance: &Instance, surface: &Surface) -> Result<Self, Box<dyn std::error::Error>> {
        let physical_devices = unsafe { instance.instance.enumerate_physical_devices()? };
        if physical_devices.is_empty() {
            return Err("No Vulkan physical devices found".into());
        }

        // Find a suitable device with graphics and present queues
        let mut selected_device = None;

        for &pdevice in &physical_devices {
            let queue_familie_properties = unsafe {
                instance
                    .instance
                    .get_physical_device_queue_family_properties(pdevice)
            };

            let graphics_queue =
                queue_familie_properties
                    .iter()
                    .enumerate()
                    .find_map(|(idx, props)| {
                        if props.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                            Some(idx as u32)
                        } else {
                            None
                        }
                    });

            let present_queue =
                queue_familie_properties
                    .iter()
                    .enumerate()
                    .find_map(|(idx, _props)| {
                        let supports_present = unsafe {
                            surface
                                .loader
                                .get_physical_device_surface_support(
                                    pdevice,
                                    idx as u32,
                                    surface.surface,
                                )
                                .unwrap_or(false)
                        };
                        if supports_present {
                            Some(idx as u32)
                        } else {
                            None
                        }
                    });

            if let (Some(graphics), Some(present)) = (graphics_queue, present_queue) {
                let props = unsafe { instance.instance.get_physical_device_properties(pdevice) };
                let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) };
                println!("Selected device: {:?}", name);

                selected_device = Some((pdevice, graphics, present));
                break;
            }
        }

        let (physical_device, graphics_queue_family_idx, present_queue_family_idx) =
            selected_device.ok_or("No suitable physical device found")?;

        // Create logical device
        let queue_priorities = [1.0f32];

        // Create unique queue families
        let mut unique_queue_families = vec![graphics_queue_family_idx];
        if present_queue_family_idx != graphics_queue_family_idx {
            unique_queue_families.push(present_queue_family_idx);
        }

        let queue_create_infos: Vec<_> = unique_queue_families
            .iter()
            .map(|&family_idx| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(family_idx)
                    .queue_priorities(&queue_priorities)
            })
            .collect();

        let device_features = vk::PhysicalDeviceFeatures::default();

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(DEVICE_EXTENSIONS)
            .enabled_features(&device_features);

        let device = unsafe {
            instance
                .instance
                .create_device(physical_device, &device_create_info, None)?
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_idx, 0) };
        let present_queue = unsafe { device.get_device_queue(present_queue_family_idx, 0) };

        Ok(Self {
            physical_device,
            device,

            graphics_queue_family_idx,
            graphics_queue,

            present_queue_family_idx,
            present_queue,
        })
    }
}

pub struct Swapchain {
    pub loader: khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
    device: ash::Device, // Device is only 48 bytes wrapper (safe to clone if cleanup done correctly)
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for &view in &self.image_views {
                self.device.destroy_image_view(view, None);
            }
            self.loader.destroy_swapchain(self.swapchain, None);
        }
    }
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        device: &Device,
        surface: &Surface,
        window: &winit::window::Window,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let surface_capabilities = unsafe {
            surface
                .loader
                .get_physical_device_surface_capabilities(device.physical_device, surface.surface)?
        };

        // Query color formats supported by surface
        let surface_formats = unsafe {
            surface
                .loader
                .get_physical_device_surface_formats(device.physical_device, surface.surface)?
        };

        // Query supported presentation modes
        let present_modes = unsafe {
            surface.loader.get_physical_device_surface_present_modes(
                device.physical_device,
                surface.surface,
            )?
        };

        // Choose surface format
        let format = surface_formats
            .iter()
            .find(|&f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .copied()
            .unwrap_or_else(|| panic!("Not supported format found"));

        // Choose present mode (prefer mailbox for lower latency)
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            // FIFO is guaranteed on all GPUs
            .unwrap_or(vk::PresentModeKHR::FIFO);

        // Choose extent
        let extent = if surface_capabilities.current_extent.width != u32::MAX {
            // the extent is defined by windowing system
            surface_capabilities.current_extent
        } else {
            // Get size from the winit
            let size = window.inner_size();
            // Create extent clamped to surface capabilities
            vk::Extent2D {
                width: size.width.clamp(
                    surface_capabilities.min_image_extent.width,
                    surface_capabilities.max_image_extent.width,
                ),
                height: size.height.clamp(
                    surface_capabilities.min_image_extent.height,
                    surface_capabilities.max_image_extent.height,
                ),
            }
        };

        let image_count = (surface_capabilities.min_image_count + 1).min(
            if surface_capabilities.max_image_count > 0 {
                surface_capabilities.max_image_count
            } else {
                u32::MAX
            },
        );

        // let queue_family_indices = &[
        //     device.graphics_queue_family_idx,
        //     device.present_queue_family_idx,
        // ];

        let (image_sharing_mode, queue_family_indices) =
            if device.graphics_queue_family_idx == device.present_queue_family_idx {
                (vk::SharingMode::EXCLUSIVE, vec![])
            } else {
                (
                    vk::SharingMode::CONCURRENT,
                    vec![
                        device.graphics_queue_family_idx,
                        device.present_queue_family_idx,
                    ],
                )
            };

        let mut swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        if image_sharing_mode == vk::SharingMode::CONCURRENT {
            swapchain_create_info =
                swapchain_create_info.queue_family_indices(&queue_family_indices);
        }

        if let Some(old_swapchain) = old_swapchain {
            swapchain_create_info = swapchain_create_info.old_swapchain(old_swapchain);
        }

        let loader = khr::swapchain::Device::new(&instance.instance, &device.device);
        let swapchain = unsafe { loader.create_swapchain(&swapchain_create_info, None)? };

        let images = unsafe { loader.get_swapchain_images(swapchain)? };

        let image_views: Vec<_> = images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
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

                unsafe { device.device.create_image_view(&create_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        println!(
            "Created swapchain: {}x{}, {} images",
            extent.width,
            extent.height,
            images.len()
        );

        Ok(Self {
            loader,
            swapchain,
            images,
            image_views,
            format,
            extent,
            device: device.device.clone(),
        })
    }

    fn recreate(self, window: &winit::window::Window) -> Result<(), Box<dyn std::error::Error>> {
        // Wait for all GPU operations to complete before destroying resources
        unsafe { self.device.device_wait_idle() };

        todo!()
    }
}
