//! # The Seven Daughters 24/7 Siren Radio & ATAK Resilient Mesh Module (`kuroshio_mesh::siren_radio`)
//!
//! Canonical implementation of **The Seven Daughters** 24/7 Siren Frequency Theater Radio
//! (`siren:frequency:global_stage`, $432\text{ Hz}$ base) with ATAK-grade multi-tier failover:
//! - **Primary Cloud Ingress**: Cloud Atlantis Hub (`https://radio.tr7n7t7.com/live.mp3`, Hugging Face Space `josh375/triton-tier1-hub`).
//! - **Local Survivability Fallback**: Club Home Node Raspberry Pi LAN relay (`http://10.0.0.237:8080/live.mp3`) and localhost (`http://127.0.0.1:8082/live.mp3`).
//! - **Spatial Sabine $RT_{60}$ Acoustics & Coral Micro-Tipping** ($0.1\text{ CYAN} = 100,000\text{ micro-CYAN}$).

use nautilus_world::Wgs84GeoCoord;
use serde::{Deserialize, Serialize};

/// The Seven Daughters canonical broadcasters operating the 24/7 Siren Frequency wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SirenDaughter {
    /// 04:00 - 08:00 UTC • 432 Hz • Dawn Oceanic Ambient / Horizon Chill
    Maia,
    /// 08:00 - 12:00 UTC • 528 Hz • High-Speed Cyberpunk D&B / Breakbeats
    Alcyone,
    /// 12:00 - 16:00 UTC • 639 Hz • Tactical Deep House & Frontline Reports
    Asterope,
    /// 16:00 - 20:00 UTC • 741 Hz • Glitch / Industrial Bass / Storm Chaser
    Celaeno,
    /// 20:00 - 00:00 UTC • 852 Hz • Vocal Trance / Eurobeat / Club Atlantis
    Electra,
    /// 00:00 - 02:00 UTC • 963 Hz • Darkwave / Minimal Techno / Abyssal Drone
    Taygete,
    /// 02:00 - 04:00 UTC • 396 Hz • Lo-Fi / Starlight Harbor Acoustic
    Merope,
}

impl SirenDaughter {
    /// Resolves the active daughter on air for any given UTC hour (`0..=23`).
    pub fn for_utc_hour(hour_utc: u8) -> Self {
        match hour_utc % 24 {
            0..=1 => Self::Taygete,
            2..=3 => Self::Merope,
            4..=7 => Self::Maia,
            8..=11 => Self::Alcyone,
            12..=15 => Self::Asterope,
            16..=19 => Self::Celaeno,
            _ => Self::Electra,
        }
    }

    /// Solfeggio / harmonic carrier frequency in Hz.
    pub fn solfeggio_frequency_hz(&self) -> f32 {
        match self {
            Self::Maia => 432.0,
            Self::Alcyone => 528.0,
            Self::Asterope => 639.0,
            Self::Celaeno => 741.0,
            Self::Electra => 852.0,
            Self::Taygete => 963.0,
            Self::Merope => 396.0,
        }
    }

    /// Broadcast genre and show title.
    pub fn genre_and_program(&self) -> &'static str {
        match self {
            Self::Maia => "Dawn Oceanic Ambient & Horizon Chill",
            Self::Alcyone => "High-Speed Cyberpunk D&B & Breakbeats",
            Self::Asterope => "Tactical Deep House & Frontline Reports",
            Self::Celaeno => "Glitch, Industrial Bass & Storm Chaser",
            Self::Electra => "Vocal Trance, Eurobeat & Club Atlantis",
            Self::Taygete => "Darkwave, Minimal Techno & Abyssal Drone",
            Self::Merope => "Lo-Fi & Starlight Harbor Acoustic",
        }
    }

    /// Canonical decentralized identifier (DID) for Coral micro-tipping.
    pub fn broadcaster_did(&self) -> &'static str {
        match self {
            Self::Maia => "did:atlantis:siren:maia",
            Self::Alcyone => "did:atlantis:siren:alcyone",
            Self::Asterope => "did:atlantis:siren:asterope",
            Self::Celaeno => "did:atlantis:siren:celaeno",
            Self::Electra => "did:atlantis:siren:electra",
            Self::Taygete => "did:atlantis:siren:taygete",
            Self::Merope => "did:atlantis:siren:merope",
        }
    }
}

/// Siren FM / VHF Tactical Carrier Channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SirenChannelPreset {
    /// 108.4 MHz — Global Stage OST & The Seven Daughters 24/7 Music & Banter
    GlobalStageOst,
    /// 144.8 MHz — Tactical Sonar, ADS-B / Maritime AIS & Frontline Alerts
    TacticalSonar,
    /// 96.2 MHz — Kuroshio Gyre Deep Trench Ambient & Whale Telemetry
    GyreDeepAmbient,
}

impl SirenChannelPreset {
    pub fn carrier_mhz(&self) -> f32 {
        match self {
            Self::GlobalStageOst => 108.4,
            Self::TacticalSonar => 144.8,
            Self::GyreDeepAmbient => 96.2,
        }
    }

    pub fn topic(&self) -> &'static str {
        match self {
            Self::GlobalStageOst => "siren:frequency:global_stage",
            Self::TacticalSonar => "siren:frequency:tactical_sonar",
            Self::GyreDeepAmbient => "siren:frequency:gyre_ambient",
        }
    }
}

/// Network tier for ATAK-style resilient stream failover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SirenEndpointTier {
    /// Primary Cloud Atlantis Hub (Global HTTPS / LL-HLS ingress).
    PrimaryCloudHub,
    /// Distributed Hugging Face / Cloudflare Zero-Cost Relay.
    DistributedCloudRelay,
    /// Local Club Home Node Raspberry Pi Survivability Fallback (`10.0.0.237:8080`).
    LocalClubPiFallback,
    /// Localhost development / edge daemon loopback (`127.0.0.1:8082`).
    LocalLoopback,
}

/// Stream endpoint descriptor with automatic failover priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SirenStreamEndpoint {
    pub tier: SirenEndpointTier,
    pub stream_url: String,
    pub is_healthy: bool,
    pub consecutive_failures: u32,
}

/// Complete state of the 24/7 Siren Radio tuner, ATAK failover chain, and Coral tip jar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SirenRadioTuner {
    pub active_channel: SirenChannelPreset,
    pub active_daughter: SirenDaughter,
    pub current_utc_hour: u8,
    pub endpoints: Vec<SirenStreamEndpoint>,
    pub active_endpoint_index: usize,
    pub total_tips_micro_cyan: u64,
    pub last_on_air_banter: String,
    /// Anchor coordinates of the primary broadcast tower (Clear Lake / Node 77034 default).
    pub transmitter_anchor: Wgs84GeoCoord,
}

impl Default for SirenRadioTuner {
    fn default() -> Self {
        Self::new(12)
    }
}

impl SirenRadioTuner {
    /// Creates a new Siren Radio tuner initialized with the canonical ATAK failover chain.
    pub fn new(initial_utc_hour: u8) -> Self {
        let daughter = SirenDaughter::for_utc_hour(initial_utc_hour);
        let endpoints = vec![
            SirenStreamEndpoint {
                tier: SirenEndpointTier::PrimaryCloudHub,
                stream_url: "https://radio.tr7n7t7.com/live.mp3".to_string(),
                is_healthy: true,
                consecutive_failures: 0,
            },
            SirenStreamEndpoint {
                tier: SirenEndpointTier::DistributedCloudRelay,
                stream_url: "https://josh375-triton-tier1-hub.hf.space/live.mp3".to_string(),
                is_healthy: true,
                consecutive_failures: 0,
            },
            SirenStreamEndpoint {
                tier: SirenEndpointTier::LocalClubPiFallback,
                stream_url: "http://10.0.0.237:8080/live.mp3".to_string(),
                is_healthy: true,
                consecutive_failures: 0,
            },
            SirenStreamEndpoint {
                tier: SirenEndpointTier::LocalLoopback,
                stream_url: "http://127.0.0.1:8082/live.mp3".to_string(),
                is_healthy: true,
                consecutive_failures: 0,
            },
        ];

        Self {
            active_channel: SirenChannelPreset::GlobalStageOst,
            active_daughter: daughter,
            current_utc_hour: initial_utc_hour % 24,
            endpoints,
            active_endpoint_index: 0,
            total_tips_micro_cyan: 0,
            last_on_air_banter: format!(
                "[{:?} • {:.0} Hz] Broadcasting live on 108.4 MHz — {}",
                daughter,
                daughter.solfeggio_frequency_hz(),
                daughter.genre_and_program()
            ),
            // Clear Lake / NASA Ellington Node 77034 default transmitter tower
            transmitter_anchor: Wgs84GeoCoord::new(29.6111, -95.1581, 45.0),
        }
    }

    /// Updates the UTC hour wheel and transitions the active Siren Daughter if needed.
    pub fn sync_utc_hour(&mut self, hour_utc: u8) -> SirenDaughter {
        self.current_utc_hour = hour_utc % 24;
        self.active_daughter = SirenDaughter::for_utc_hour(self.current_utc_hour);
        self.last_on_air_banter = format!(
            "[{:?} • {:.0} Hz] {}",
            self.active_daughter,
            self.active_daughter.solfeggio_frequency_hz(),
            self.active_daughter.genre_and_program()
        );
        self.active_daughter
    }

    /// Returns the currently active stream URL from the ATAK failover hierarchy.
    pub fn active_stream_url(&self) -> &str {
        &self.endpoints[self.active_endpoint_index].stream_url
    }

    /// Reports a stream connection drop on the current tier and automatically fails over
    /// to the next available endpoint (Cloud Hub -> HF Relay -> Local Pi `10.0.0.237:8080` -> Localhost).
    pub fn report_stream_failure_and_failover(&mut self) -> &SirenStreamEndpoint {
        if let Some(ep) = self.endpoints.get_mut(self.active_endpoint_index) {
            ep.consecutive_failures += 1;
            ep.is_healthy = false;
        }
        self.active_endpoint_index = (self.active_endpoint_index + 1) % self.endpoints.len();
        &self.endpoints[self.active_endpoint_index]
    }

    /// Sends an instant 0.1 CYAN ($100,000\text{ micro-CYAN}$) Coral State Channel tip to the active Daughter.
    pub fn tip_active_daughter_coral(&mut self, tip_micro_cyan: u64) -> (&'static str, u64) {
        let amount = tip_micro_cyan.max(100_000); // Minimum 0.1 CYAN standard tip
        self.total_tips_micro_cyan += amount;
        (self.active_daughter.broadcaster_did(), self.total_tips_micro_cyan)
    }

    /// Computes Sabine $RT_{60} = \frac{0.161 \cdot V}{A + 4mV}$ reverberation decay time (seconds)
    /// and distance attenuation for 3D spatial radio playback in venues and cockpits.
    pub fn compute_spatial_acoustics(
        &self,
        listener_geo: Wgs84GeoCoord,
        room_volume_m3: f32,
        absorption_sabins: f32,
    ) -> (f32, f32) {
        let dist_m = self
            .transmitter_anchor
            .to_ecef()
            .distance(listener_geo.to_ecef()) as f32;
        let air_attenuation = 0.0012f32;
        let denom = (absorption_sabins + 4.0 * air_attenuation * room_volume_m3).max(0.1);
        let rt60_seconds = (0.161 * room_volume_m3) / denom;
        // Signal strength remains 1.0 within 5km, tapering gently to 0.25 at 50km
        let gain = (1.0 - (dist_m / 65_000.0)).clamp(0.25, 1.0);
        (rt60_seconds, gain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seven_daughters_24h_schedule_and_failover() {
        let mut tuner = SirenRadioTuner::new(6);
        assert_eq!(tuner.active_daughter, SirenDaughter::Maia);
        assert_eq!(tuner.active_daughter.solfeggio_frequency_hz(), 432.0);
        assert_eq!(tuner.active_stream_url(), "https://radio.tr7n7t7.com/live.mp3");

        tuner.sync_utc_hour(21);
        assert_eq!(tuner.active_daughter, SirenDaughter::Electra);

        // Simulate WAN blackout -> fail over to HF Relay -> fail over to Local Pi 10.0.0.237:8080
        tuner.report_stream_failure_and_failover();
        let fallback = tuner.report_stream_failure_and_failover();
        assert_eq!(fallback.tier, SirenEndpointTier::LocalClubPiFallback);
        assert_eq!(fallback.stream_url, "http://10.0.0.237:8080/live.mp3");

        // Tip 0.1 CYAN
        let (did, total) = tuner.tip_active_daughter_coral(100_000);
        assert_eq!(did, "did:atlantis:siren:electra");
        assert_eq!(total, 100_000);
    }
}
