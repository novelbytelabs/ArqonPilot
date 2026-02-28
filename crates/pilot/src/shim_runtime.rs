use std::env;

pub fn bus_shim_report_dir() -> String {
    env::var("PILOT_REPORT_DIR").unwrap_or_else(|_| "/tmp/pilot-reports".to_string())
}

pub fn bus_shim_command(action: &str) -> String {
    let report_dir = bus_shim_report_dir();
    match action {
        "start" => format!(
            "PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh start && PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh status",
            report_dir, report_dir
        ),
        "stop" => format!(
            "PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh stop && PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh status",
            report_dir, report_dir
        ),
        "restart" => format!(
            "PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh stop || true; PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh start && PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh status",
            report_dir, report_dir, report_dir
        ),
        _ => format!(
            "PILOT_REPORT_DIR={} ./scripts/arqonbus_shim.sh status",
            report_dir
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{bus_shim_command, bus_shim_report_dir};

    #[test]
    fn test_bus_shim_report_dir_default() {
        std::env::remove_var("PILOT_REPORT_DIR");
        assert_eq!(bus_shim_report_dir(), "/tmp/pilot-reports");
    }

    #[test]
    fn test_bus_shim_command_contains_expected_action() {
        let start = bus_shim_command("start");
        assert!(start.contains("PILOT_REPORT_DIR="));
        assert!(start.contains("arqonbus_shim.sh start"));
        assert!(start.contains("arqonbus_shim.sh status"));
        let stop = bus_shim_command("stop");
        assert!(stop.contains("arqonbus_shim.sh stop"));
        let restart = bus_shim_command("restart");
        assert!(restart.contains("arqonbus_shim.sh stop || true"));
        let status = bus_shim_command("status");
        assert!(status.contains("arqonbus_shim.sh status"));
    }
}
