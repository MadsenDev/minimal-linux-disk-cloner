use crate::models::{DiskDevice, WarningItem, WarningSeverity};

pub fn build_clone_warnings(source: &DiskDevice, target: &DiskDevice) -> Vec<WarningItem> {
    let mut warnings = Vec::new();

    if source.path == target.path {
        warnings.push(WarningItem {
            severity: WarningSeverity::Danger,
            code: "same-device",
            message: "Source and target are the same device. This operation is not allowed."
                .to_owned(),
        });
    }

    if target.size_bytes < source.size_bytes {
        let shortfall = source.size_bytes - target.size_bytes;
        warnings.push(WarningItem {
            severity: WarningSeverity::Danger,
            code: "smaller-target",
            message: format!(
                "Target is smaller than source by {} ({} vs {}). `dd` will likely fail before completion.",
                format_warning_size(shortfall),
                format_warning_size(target.size_bytes),
                format_warning_size(source.size_bytes),
            ),
        });
    }

    if !source.mounted_partitions.is_empty() {
        warnings.push(WarningItem {
            severity: WarningSeverity::Warning,
            code: "source-mounted",
            message:
                "Source has mounted partitions. The clone may capture an inconsistent live state."
                    .to_owned(),
        });
    }

    if !target.mounted_partitions.is_empty() {
        warnings.push(WarningItem {
            severity: WarningSeverity::Danger,
            code: "target-mounted",
            message: "Target has mounted partitions. Writes may conflict with the running system."
                .to_owned(),
        });
    }

    if target.removable {
        warnings.push(WarningItem {
            severity: WarningSeverity::Warning,
            code: "target-removable",
            message: "Target is marked as removable. Confirm that you selected the correct device."
                .to_owned(),
        });
    }

    if source.removable {
        warnings.push(WarningItem {
            severity: WarningSeverity::Info,
            code: "source-removable",
            message: "Source is marked as removable.".to_owned(),
        });
    }

    if source.model.is_none() || source.serial.is_none() {
        warnings.push(WarningItem {
            severity: WarningSeverity::Info,
            code: "source-metadata",
            message: "Source is missing some metadata. Rely on the device path and size before proceeding."
                .to_owned(),
        });
    }

    if target.model.is_none() || target.serial.is_none() {
        warnings.push(WarningItem {
            severity: WarningSeverity::Info,
            code: "target-metadata",
            message:
                "Target is missing some metadata. Re-check the device path and size carefully."
                    .to_owned(),
        });
    }

    warnings
}

fn format_warning_size(bytes: u64) -> String {
    let gib = bytes as f64 / 1024_f64.powi(3);
    format!("{gib:.2} GiB ({bytes} bytes)")
}

#[cfg(test)]
mod tests {
    use crate::models::{DiskDevice, PartitionInfo};

    #[test]
    fn generates_expected_warning_set() {
        let source = DiskDevice {
            path: "/dev/sda".to_owned(),
            name: "sda".to_owned(),
            size_bytes: 2_000,
            size_human: "2.0 KiB".to_owned(),
            model: None,
            serial: None,
            removable: false,
            transport: Some("sata".to_owned()),
            mounted_partitions: vec![PartitionInfo {
                path: "/dev/sda1".to_owned(),
                mountpoint: Some("/".to_owned()),
                fstype: Some("ext4".to_owned()),
            }],
        };
        let target = DiskDevice {
            path: "/dev/sdb".to_owned(),
            name: "sdb".to_owned(),
            size_bytes: 1_000,
            size_human: "1000 B".to_owned(),
            model: Some("USB SSD".to_owned()),
            serial: None,
            removable: true,
            transport: Some("usb".to_owned()),
            mounted_partitions: vec![PartitionInfo {
                path: "/dev/sdb1".to_owned(),
                mountpoint: Some("/media/target".to_owned()),
                fstype: Some("ext4".to_owned()),
            }],
        };

        let warnings = super::build_clone_warnings(&source, &target);

        assert!(warnings.iter().any(|w| w.code == "smaller-target"));
        assert!(warnings.iter().any(|w| w.code == "source-mounted"));
        assert!(warnings.iter().any(|w| w.code == "target-mounted"));
        assert!(warnings.iter().any(|w| w.code == "target-removable"));
    }

    #[test]
    fn smaller_target_warning_uses_precise_sizes() {
        let source = DiskDevice {
            path: "/dev/sda".to_owned(),
            name: "sda".to_owned(),
            size_bytes: 62_085_935_104,
            size_human: "57.8 GiB".to_owned(),
            model: Some("USB A".to_owned()),
            serial: Some("SRC".to_owned()),
            removable: true,
            transport: Some("usb".to_owned()),
            mounted_partitions: Vec::new(),
        };
        let target = DiskDevice {
            path: "/dev/sdb".to_owned(),
            name: "sdb".to_owned(),
            size_bytes: 62_085_410_816,
            size_human: "57.8 GiB".to_owned(),
            model: Some("USB B".to_owned()),
            serial: Some("DST".to_owned()),
            removable: true,
            transport: Some("usb".to_owned()),
            mounted_partitions: Vec::new(),
        };

        let warnings = super::build_clone_warnings(&source, &target);
        let smaller_target = warnings
            .iter()
            .find(|warning| warning.code == "smaller-target")
            .expect("expected smaller-target warning");

        assert!(smaller_target.message.contains("524288 bytes"));
        assert!(smaller_target.message.contains("62085410816 bytes"));
        assert!(smaller_target.message.contains("62085935104 bytes"));
        assert!(smaller_target.message.contains("bytes"));
    }
}
