#![allow(unused)]

use ash::vk;

fn select_physical_device(
    instance: &ash::Instance,
    physical_devices: Vec<vk::PhysicalDevice>,
) -> Result<vk::PhysicalDevice, vk::Result> {
    #[allow(clippy::never_loop)]
    for physical_device in physical_devices {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let physical_device_properties =
            unsafe { instance.get_physical_device_properties(physical_device) };

        return Ok(physical_device);
    }
    todo!()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entry = unsafe { ash::Entry::load()? };
    let application_info = vk::ApplicationInfo::default();
    let instance_create_info =
        vk::InstanceCreateInfo::default().application_info(&application_info);
    let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

    let physical_devices = unsafe { instance.enumerate_physical_devices()? };
    let physical_device = select_physical_device(&instance, physical_devices)?;

    Ok(())
}
