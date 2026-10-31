pub mod byedpi_runner;
pub mod core_runner;
pub mod engine;
pub mod latency;
pub mod sub_updater;
pub mod system_proxy;

pub use byedpi_runner::{resolve_ciadpi_bin, ByeDpiPreset, ByeDpiRunner};
pub use core_runner::{resolve_core_bin, CoreRunner};
pub use engine::ThroneEngine;
pub use latency::measure_tcp_latency;
pub use sub_updater::SubUpdater;
pub use system_proxy::SystemProxyManager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byedpi_presets() {
        let std = ByeDpiPreset::from_str_name("standard");
        assert_eq!(std.to_args(), vec!["-r", "1+s"]);

        let vf = ByeDpiPreset::from_str_name("vodafone");
        assert_eq!(
            vf.to_args(),
            vec!["-s1", "-At", "-d2", "-f-1", "-r1+s", "-An"]
        );
    }

    #[test]
    fn test_byedpi_bin_resolution() {
        // In this environment, /home/kazim/byedpi_throne_linux/ciadpi exists
        let bin = resolve_ciadpi_bin(None);
        assert!(bin.is_some(), "Expected to locate ciadpi binary");
    }

    #[test]
    fn test_core_bin_resolution() {
        let bin = resolve_core_bin(None);
        assert!(bin.exists(), "Expected to locate sing-box binary, found: {:?}", bin);
    }
}
