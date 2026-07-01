//! Cross-module constants only

// Suspicious-expression gaze offset (-1..1)
pub const SUSPICIOUS_GAZE_OFFSET: f32 = 0.7;

// FFT buffer size
pub const FFT_BUFFER_SIZE: usize = 64;

// Rarity percentages roll modulo this eye (sin) + reactor (reactions)
pub const RARITY_SCALE: u32 = 100;

// Default config values
pub const DEFAULT_PUPIL_HEIGHT: i32 = 70;
pub const DEFAULT_WIGGLE_AMPLITUDE: i32 = 140;
pub const DEFAULT_BLINK_INTERVAL: u32 = 180;
pub const DEFAULT_EYE_RED: i32 = 26;
pub const DEFAULT_EYE_GREEN: i32 = 5;
pub const DEFAULT_EYE_BLUE: i32 = 0;
pub const DEFAULT_SOUND_THRESHOLD: i32 = 40;
pub const DEFAULT_SIN_RARITY: u32 = 1; // % chance after each blink (0 = off)
pub const DEFAULT_EYEROLL_RARITY: u32 = 15;
pub const DEFAULT_STARTLED_RARITY: u32 = 35;
pub const DEFAULT_SUSPICIOUS_RARITY: u32 = 28;
pub const DEFAULT_ANGRY_RARITY: u32 = 55;

// Captive portal network
pub const GATEWAY_IP: core::net::Ipv4Addr = core::net::Ipv4Addr::new(192, 168, 4, 1);
pub const PACKET_META_SLOTS: usize = 4;
