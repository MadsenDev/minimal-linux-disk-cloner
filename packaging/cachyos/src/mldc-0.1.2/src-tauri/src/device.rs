use std::process::Command;

use serde::Deserialize;

use crate::models::{DiskDevice, PartitionInfo};

#[derive(Debug)]
pub struct DeviceError {
    pub message: String,
}

impl DeviceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LsblkOutput {
    #[serde(default)]
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Clone, Debug, Deserialize)]
struct LsblkDevice {
    name: String,
    path: Option<String>,
    size: Option<u64>,
    model: Option<String>,
    serial: Option<String>,
    #[serde(rename = "type")]
    device_type: String,
    rm: Option<bool>,
    tran: Option<String>,
    mountpoint: Option<String>,
    fstype: Option<String>,
    #[serde(default)]
    children: Vec<LsblkDevice>,
}

pub fn ensure_tool_available(tool: &str) -> Result<(), DeviceError> {
    let status = Command::new(tool)
        .arg("--version")
        .status()
        .map_err(|err| DeviceError::new(format!("Failed to execute `{tool}`: {err}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(DeviceError::new(format!(
            "`{tool}` is present but returned a non-zero status for --version"
        )))
    }
}

pub fn discover_disks() -> Result<Vec<DiskDevice>, DeviceError> {
    ensure_tool_available("lsblk")?;

    let output = Command::new("lsblk")
        .args([
            "-J",
            "-b",
            "-o",
            "NAME,PATH,SIZE,MODEL,SERIAL,TYPE,RM,TRAN,MOUNTPOINT,FSTYPE",
        ])
        .output()
        .map_err(|err| DeviceError::new(format!("Failed to run `lsblk`: {err}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DeviceError::new(format!(
            "`lsblk` failed: {}",
            stderr.trim()
        )));
    }

    let parsed: LsblkOutput = serde_json::from_slice(&output.stdout)
        .map_err(|err| DeviceError::new(format!("Failed to parse `lsblk` JSON: {err}")))?;

    let mut devices: Vec<DiskDevice> = parsed
        .blockdevices
        .into_iter()
        .filter(|device| device.device_type == "disk")
        .filter(|device| !is_filtered_disk_name(&device.name))
        .map(normalize_disk)
        .collect();

    devices.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(devices)
}

fn normalize_disk(device: LsblkDevice) -> DiskDevice {
    let mounted_partitions = collect_partitions(&device);
    let size_bytes = device.size.unwrap_or(0);

    DiskDevice {
        path: device
            .path
            .unwrap_or_else(|| format!("/dev/{}", device.name)),
        name: device.name,
        size_bytes,
        size_human: crate::models::format_bytes(size_bytes),
        model: trim_optional(device.model),
        serial: trim_optional(device.serial),
        removable: device.rm.unwrap_or(false),
        transport: trim_optional(device.tran),
        mounted_partitions,
    }
}

fn collect_partitions(device: &LsblkDevice) -> Vec<PartitionInfo> {
    let mut partitions = Vec::new();
    collect_partitions_recursive(&device.children, &mut partitions);
    partitions
}

fn collect_partitions_recursive(children: &[LsblkDevice], out: &mut Vec<PartitionInfo>) {
    for child in children {
        if child.device_type == "part" {
            let mountpoint = trim_optional(child.mountpoint.clone());
            if mountpoint.is_some() {
                out.push(PartitionInfo {
                    path: child
                        .path
                        .clone()
                        .unwrap_or_else(|| format!("/dev/{}", child.name)),
                    mountpoint,
                    fstype: trim_optional(child.fstype.clone()),
                });
            }
        }

        if !child.children.is_empty() {
            collect_partitions_recursive(&child.children, out);
        }
    }
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim().to_owned();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn is_filtered_disk_name(name: &str) -> bool {
    name.starts_with("loop")
        || name.starts_with("ram")
        || name.starts_with("zram")
        || name.starts_with("sr")
}

#[cfg(test)]
mod tests {
    use super::LsblkOutput;

    fn sample() -> &'static str {
        r#"
        {
          "blockdevices": [
            {
              "name": "loop0",
              "path": "/dev/loop0",
              "size": 4096,
              "model": null,
              "serial": null,
              "type": "loop",
              "rm": false,
              "tran": null,
              "mountpoint": null,
              "fstype": null,
              "children": []
            },
            {
              "name": "nvme0n1",
              "path": "/dev/nvme0n1",
              "size": 1000204886016,
              "model": "Samsung SSD",
              "serial": "ABC123",
              "type": "disk",
              "rm": false,
              "tran": "nvme",
              "mountpoint": null,
              "fstype": null,
              "children": [
                {
                  "name": "nvme0n1p1",
                  "path": "/dev/nvme0n1p1",
                  "size": 536870912,
                  "model": null,
                  "serial": null,
                  "type": "part",
                  "rm": false,
                  "tran": null,
                  "mountpoint": "/boot",
                  "fstype": "vfat",
                  "children": []
                }
              ]
            },
            {
              "name": "sdb",
              "path": "/dev/sdb",
              "size": 62008590336,
              "model": "USB Stick",
              "serial": null,
              "type": "disk",
              "rm": true,
              "tran": "usb",
              "mountpoint": null,
              "fstype": null,
              "children": [
                {
                  "name": "sdb1",
                  "path": "/dev/sdb1",
                  "size": 62007541760,
                  "model": null,
                  "serial": null,
                  "type": "part",
                  "rm": true,
                  "tran": null,
                  "mountpoint": null,
                  "fstype": "vfat",
                  "children": []
                }
              ]
            }
          ]
        }
        "#
    }

    fn discover_disks_from_output(output: LsblkOutput) -> Vec<crate::models::DiskDevice> {
        let mut devices: Vec<_> = output
            .blockdevices
            .into_iter()
            .filter(|device| device.device_type == "disk")
            .filter(|device| !super::is_filtered_disk_name(&device.name))
            .map(super::normalize_disk)
            .collect();
        devices.sort_by(|a, b| a.path.cmp(&b.path));
        devices
    }

    #[test]
    fn parses_and_filters_cloneable_disks() {
        let parsed: LsblkOutput = serde_json::from_str(sample()).unwrap();
        let devices = discover_disks_from_output(parsed);

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].path, "/dev/nvme0n1");
        assert_eq!(devices[0].mounted_partitions.len(), 1);
        assert_eq!(
            devices[0].mounted_partitions[0].mountpoint.as_deref(),
            Some("/boot")
        );
        assert_eq!(devices[1].path, "/dev/sdb");
        assert!(devices[1].mounted_partitions.is_empty());
    }
}
