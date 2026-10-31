pub mod error;
pub mod generator;
pub mod models;
pub mod parser;
pub mod protocols;

pub use error::{CoreError, Result};
pub use generator::SingBoxGenerator;
pub use models::{Group, Profile, RouteProfile, RouteRule, Settings};
pub use parser::{parse_clash, parse_subscription, parse_uri};
pub use protocols::OutboundConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vless_uri() {
        let uri = "vless://a1b2c3d4-e5f6-7890-abcd-ef1234567890@example.com:443?encryption=none&security=reality&sni=example.com&fp=chrome&pbk=1234567890abcdef1234567890abcdef1234567890ab&sid=12345678&type=tcp&headerType=none&flow=xtls-rprx-vision#MyNode";
        let (name, config) = parse_uri(uri).expect("Failed to parse vless uri");
        assert_eq!(name, "MyNode");
        match config {
            OutboundConfig::Vless(v) => {
                assert_eq!(v.server, "example.com");
                assert_eq!(v.server_port, 443);
                assert_eq!(v.uuid, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
                assert_eq!(v.flow, Some("xtls-rprx-vision".to_string()));
                assert!(v.tls.enabled);
                assert!(v.tls.reality.is_some());
            }
            _ => panic!("Expected VLESS outbound"),
        }
    }

    #[test]
    fn test_parse_ss_sip002() {
        let uri = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmRAMTIz@192.168.1.1:8388#TestSS";
        let (name, config) = parse_uri(uri).expect("Failed to parse ss uri");
        assert_eq!(name, "TestSS");
        match config {
            OutboundConfig::Shadowsocks(s) => {
                assert_eq!(s.server, "192.168.1.1");
                assert_eq!(s.server_port, 8388);
                assert_eq!(s.method, "aes-256-gcm");
                assert_eq!(s.password, "password@123");
            }
            _ => panic!("Expected Shadowsocks outbound"),
        }
    }

    #[test]
    fn test_singbox_config_generator() {
        let settings = Settings::default();
        let profile = Profile {
            outbound_json: serde_json::to_string(&OutboundConfig::Vless(protocols::VlessOutbound {
                server: "server.com".to_string(),
                server_port: 443,
                uuid: "00000000-0000-0000-0000-000000000000".to_string(),
                ..Default::default()
            }))
            .unwrap(),
            ..Default::default()
        };

        let generator = SingBoxGenerator::new(&settings, &profile, None, &[]);
        let config = generator.generate().expect("Generation failed");

        assert!(config["inbounds"].is_array());
        assert!(config["outbounds"].is_array());
        assert!(config["dns"].is_object());
        assert!(config["route"].is_object());
    }
}
