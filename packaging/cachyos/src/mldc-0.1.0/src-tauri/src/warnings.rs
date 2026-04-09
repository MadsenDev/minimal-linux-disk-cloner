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
        warnings.push(WarningItem {
            severity: WarningSeverity::Danger,
            code: "smaller-target",
            message: format!(
                "Target is smaller than source ({} vs {}). `dd` will likely fail before completion.",
                target.size_human, source.size_human
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
}
