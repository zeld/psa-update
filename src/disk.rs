use std::env::current_dir;
use std::fs;
use std::str;

use sysinfo::{Disk, DiskExt, System, SystemExt};

use console::Style;
use indicatif::DecimalBytes;

use log::debug;

// Print disks list as a table
// Requires list of disk to be up to date:
//     let mut sys: System = System::new();
//     sys.refresh_disks_list();
//     sys.refresh_disks();
pub fn print_disks(sys: &System) {
    println!(
        "{0: ^20} | {1: ^30} | {2: ^6} | {3: ^9} | {4: ^10} | {5: ^5} ",
        "Name", "Mount point", "Type", "Removable", "Avail.", "Empty"
    );
    let red = Style::new().red();
    let green = Style::new().green();
    for disk in sys.disks() {
        let disk_removable = if disk.is_removable() {
            green.apply_to("Yes")
        } else {
            red.apply_to("No")
        };
        let file_system_str = str::from_utf8(disk.file_system()).unwrap();
        let file_system = if file_system_str.eq_ignore_ascii_case("vfat")
            || file_system_str.eq_ignore_ascii_case("fat32")
        {
            green.apply_to(file_system_str)
        } else {
            red.apply_to(file_system_str)
        };

        let empty = if let Ok(files) = fs::read_dir(disk.mount_point()) {
            if files.count() == 0 {
                green.apply_to("Yes")
            } else {
                red.apply_to("No")
            }
        } else {
            red.apply_to("N/A")
        };

        println!(
            "{0: <20} | {1: <30} | {2: <6} | {3: <9} | {4: >10} | {5: <5}",
            disk.name().to_string_lossy(),
            disk.mount_point().to_string_lossy(),
            file_system,
            disk_removable,
            DecimalBytes(disk.available_space()).to_string(),
            empty
        );
    }
}

// Available disk space in current directory
// Requires list of disk to be up to date:
//     let mut sys: System = System::new();
//     sys.refresh_disks_list();
//     sys.refresh_disks();
pub fn get_current_dir_available_space(sys: &System) -> Option<u64> {
    let cwd_result = current_dir();
    if cwd_result.is_err() {
        debug!(
            "Failed to retrieve information about current working directory: {}",
            cwd_result.err().unwrap()
        );
        return None;
    }
    let cwd = cwd_result.ok().unwrap();
    let mut cwd_disk: Option<&Disk> = None;
    // Lookup disk whose mount point is parent of cwd
    // In case there are multiple candidates, pick up the "nearest" parent of cwd
    for disk in sys.disks() {
        debug!("Disk {:?}", disk);
        if cwd.starts_with(disk.mount_point())
            && (cwd_disk.is_none()
                || disk
                    .mount_point()
                    .starts_with(cwd_disk.unwrap().mount_point()))
        {
            cwd_disk = Some(disk);
        }
    }
    if cwd_disk.is_none() {
        debug!(
            "Failed to retrieve disk information for current working directory: {}",
            cwd.to_string_lossy()
        );
        return None;
    }
    debug!(
        "Current working directory maps to disk {}",
        cwd_disk.unwrap().name().to_string_lossy()
    );
    Some(cwd_disk.unwrap().available_space())
}
