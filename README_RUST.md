# Throne (Rust Port)

Throne proxy istemcisinin modern, güvenli ve performans odaklı Rust sürümü.

## 🚀 Mimari ve Crate Yapısı

Throne Rust portu, modüler bir Cargo Workspace mimarisi üzerine inşa edilmiştir:

```
Throne/
├── Cargo.toml                  # Workspace konfigürasyonu
└── crates/
    ├── throne-core/            # Protokol modelleri, URI ayrıştırıcılar ve sing-box konfigürasyon üreteci
    ├── throne-db/              # SQLite veritabanı katmanı ve repository örüntüsü
    ├── throne-api/             # ChaCha20-Poly1305 ve HMAC-SHA256 tabanlı Throne Remote API v1 protokolü
    ├── throne-service/         # sing-box süreç yöneticisi, sistem vekili (proxy), gecikme testi ve abonelik güncelleyici
    └── throne-cli/             # `throne` komut satırı arayüzü ve headless servis yöneticisi
```

### 1. `throne-core`
- **Desteklenen Protokoller:**
  - VLESS (`vless://`) - Reality, TLS, Vision akışı (`xtls-rprx-vision`), xUDP, WebSocket, gRPC, HTTPUpgrade
  - VMess (`vmess://` base64 JSON) - AEAD, TLS, WebSocket, gRPC
  - Shadowsocks (`ss://` SIP002 & legacy) - AEAD şifreleri (2022-blake3, aes-256-gcm, vb.)
  - Trojan (`trojan://`) - TLS, ALPN, WebSocket, gRPC
  - Hysteria 2 (`hy2://`, `hysteria2://`) - Salamander obfs, TLS, port atlama
  - TUIC (`tuic://`) - BBR, Cubic, UDP röle modları
  - WireGuard (`wireguard://`, `wg://`)
  - SOCKS5 ve HTTP(S)
- **Abonelik Ayrıştırma:**
  - Standart base64 abonelik listeleri
  - Clash YAML `proxies` formatı
- **Sing-Box Konfigürasyon Üretimi:**
  - `mixed` ve `tun` inbound'ları
  - Uzak ve doğrudan DNS yönlendirmesi
  - Özel kural setleri (domain, IP CIDR, özel IP bypass)

### 2. `throne-db`
- `rusqlite` ile gömülü ve sıfır bağımlılıklı SQLite motoru.
- Throne şemasına tam uyumlu tablolar:
  - `profiles`, `groups`, `route_profiles`, `route_rules`, `settings`, `traffic_stats`
- CRUD repository'leri: `ProfilesRepo`, `GroupsRepo`, `RoutesRepo`, `SettingsRepo`.

### 3. `throne-api`
- Throne Remote Wire Protocol v1 (`spec/protocol.md`):
  - Port 9095 (TCP)
  - 16 baytlık Server/Client rastgele anahtar el sıkışması
  - HMAC-SHA256 oturum anahtarı türetimi (`throne-api/1 c2s`, `throne-api/1 s2c`)
  - ChaCha20-Poly1305 AEAD çerçeveleme ve 64-bit sayaç güvenliği
  - `thronectl` ve web arayüzleri ile tam JSON-RPC uyumluluğu.

### 4. `throne-service`
- **Process Supervisor:** `sing-box` sürecinin arka planda yönetimi, yeniden başlatılması ve zarif kapatılması.
- **System Proxy Manager:** Linux (GNOME/gsettings vb.) ve masaüstü ortamlarında sistem vekili entegrasyonu.
- **Latency Engine:** TCP el sıkışması ve HTTP 204 kontrolleriyle gerçek zamanlı gecikme ölçümü.
- **Subscription Updater:** Arka planda periyodik abonelik yenileme.

### 5. `throne-cli`
- Doğrudan `throne` komutuyla çalışan zengin CLI aracı.

---

## 🛠️ Kurulum ve Derleme

Gereksinimler:
- Rust 1.75+ (Cargo)
- `sing-box` (çekirdek proxy motoru olarak PATH üzerinde veya ayarlar dosyasında belirtilmiş olmalıdır)

Derleme:
```bash
cargo build --release
```

Testleri çalıştırma:
```bash
cargo test
```

---

## 📖 Kullanım Kılavuzu

### 1. Düğüm veya Abonelik Ekleme
```bash
# Tek bir VLESS düğümü ekleme
throne import "vless://uuid@server:443?security=reality&sni=example.com&pbk=...#MyNode"

# Abonelik bağlantısı veya Clash yapılandırması ekleme
throne import "https://example.com/api/v1/client/subscribe?token=..." --group-id 1
```

### 2. Düğümleri Listeleme ve Test Etme
```bash
# Tüm kayıtlı profilleri listeleme
throne list

# Gecikme ölçümü yapma
throne test
throne test 1
```

### 3. Bağlantı Başlatma ve Kesme
```bash
# Profile ID veya ada göre bağlanma
throne connect 1
throne connect "MyNode"

# Bağlantıyı kesme
throne disconnect

# Durumu kontrol etme
throne status
```

### 4. TUN Modu ve Sistem Vekili
```bash
# TUN modunu açma / kapama
throne tun on
throne tun off

# Sistem genel vekilini açma / kapama
throne sysproxy on
throne sysproxy off
```

### 5. Headless Daemon ve Uzak API
```bash
# Arka plan servisini başlatma (port 9095)
throne daemon

# API anahtarını görüntüleme veya yenileme
throne key
throne key --regenerate
```

### 6. Sing-Box Yapılandırmasını Dışa Aktarma
```bash
throne export-config 1
```
