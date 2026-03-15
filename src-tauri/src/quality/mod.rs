//! Day 1 Experience - Zero-config onboarding
//!
//! Ensures users reach "wow moment" in <30 seconds.
//! Graceful degradation for constrained hardware.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Onboarding state machine
pub struct OnboardingManager {
    state: OnboardingState,
    experience_tier: ExperienceTier,
    wow_moment_achieved: bool,
    start_time: DateTime<Utc>,
    milestones: Vec<Milestone>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingState {
    NotStarted,
    DetectingHardware,
    Initializing,
    FirstLaunch,
    FirstFileOpen,
    FirstAIInteraction,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperienceTier {
    /// Full experience: GPU + local LLM
    Full,
    /// CPU-only with small model
    CpuOptimized,
    /// Cloud fallback
    CloudFallback,
    /// Minimal (no AI)
    Minimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub achieved_at: DateTime<Utc>,
    pub duration_ms: u64,
}

/// Hardware detection result
#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub gpu_name: Option<String>,
    pub vram_mb: u64,
    pub ram_mb: u64,
    pub cpu_cores: usize,
    pub has_avx2: bool,
    pub has_metal: bool,
    pub has_cuda: bool,
    pub tier: ExperienceTier,
}

impl OnboardingManager {
    pub fn new() -> Self {
        Self {
            state: OnboardingState::NotStarted,
            experience_tier: ExperienceTier::Full,
            wow_moment_achieved: false,
            start_time: Utc::now(),
            milestones: Vec::new(),
        }
    }

    /// Start onboarding
    pub async fn start(&mut self) -> HardwareProfile {
        self.state = OnboardingState::DetectingHardware;
        self.start_time = Utc::now();

        let profile = self.detect_hardware().await;
        self.experience_tier = profile.tier.clone();

        profile
    }

    /// Detect hardware capabilities
    async fn detect_hardware(&self) -> HardwareProfile {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();

        let ram_mb = sys.total_memory() / 1024;
        let cpu_cores = sys.cpus().len();

        // Check CPU features
        let has_avx2 = is_x86_feature_detected!("avx2");

        // Detect GPU
        let (gpu_name, vram_mb, has_metal, has_cuda) = self.detect_gpu().await;

        // Determine tier
        let tier = self.determine_tier(ram_mb, vram_mb, has_cuda, has_metal);

        HardwareProfile {
            gpu_name,
            vram_mb,
            ram_mb,
            cpu_cores,
            has_avx2,
            has_metal,
            has_cuda,
            tier,
        }
    }

    async fn detect_gpu(&self) -> (Option<String>, u64, bool, bool) {
        // Platform-specific GPU detection
        #[cfg(target_os = "macos")]
        {
            // Metal is available on Apple Silicon
            let is_apple_silicon = std::env::consts::ARCH == "aarch64";
            if is_apple_silicon {
                // Unified memory - use portion of RAM as VRAM
                let mut sys = sysinfo::System::new_all();
                sys.refresh_memory();
                let vram = sys.total_memory() / 1024 * 70 / 100; // 70% of RAM
                return (Some("Apple Silicon".to_string()), vram, true, false);
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Query NVIDIA GPU via nvidia-smi (fastest, accurate VRAM)
            if let Ok(out) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    let line = text.lines().next().unwrap_or("");
                    let parts: Vec<&str> = line.splitn(2, ',').collect();
                    if parts.len() == 2 {
                        let name = parts[0].trim().to_string();
                        let vram_mb: u64 = parts[1].trim().parse().unwrap_or(0);
                        return (Some(name), vram_mb, false, true);
                    }
                }
            }
            // Fall back to wmic for non-NVIDIA (AMD / Intel Arc)
            if let Ok(out) = std::process::Command::new("wmic")
                .args(["path", "win32_VideoController", "get", "name,AdapterRAM", "/format:csv"])
                .output()
            {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    // CSV line format: Node,AdapterRAM,Name
                    for line in text.lines().skip(1) {
                        let parts: Vec<&str> = line.splitn(3, ',').collect();
                        if parts.len() == 3 {
                            let vram_bytes: u64 = parts[1].trim().parse().unwrap_or(0);
                            let name = parts[2].trim().to_string();
                            if !name.is_empty() && vram_bytes > 0 {
                                let vram_mb = vram_bytes / 1024 / 1024;
                                let has_cuda = name.to_ascii_lowercase().contains("nvidia");
                                return (Some(name), vram_mb, false, has_cuda);
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try nvidia-smi first
            if let Ok(out) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=name,memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    let line = text.lines().next().unwrap_or("");
                    let parts: Vec<&str> = line.splitn(2, ',').collect();
                    if parts.len() == 2 {
                        let name = parts[0].trim().to_string();
                        let vram_mb: u64 = parts[1].trim().parse().unwrap_or(0);
                        return (Some(name), vram_mb, false, true);
                    }
                }
            }
            // Fall back to sysfs for AMD/Intel
            let drm_paths = [
                "/sys/class/drm/card0/device/mem_info_vram_total",
                "/sys/class/drm/card1/device/mem_info_vram_total",
            ];
            for path in &drm_paths {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(bytes) = content.trim().parse::<u64>() {
                        let vram_mb = bytes / 1024 / 1024;
                        if vram_mb > 0 {
                            let name = std::fs::read_to_string(
                                path.replace("mem_info_vram_total", "vendor")
                            ).unwrap_or_default();
                            return (Some(name.trim().to_string()), vram_mb, false, false);
                        }
                    }
                }
            }
        }

        (None, 0, false, false)
    }

    fn determine_tier(
        &self,
        ram_mb: u64,
        vram_mb: u64,
        _has_cuda: bool,
        has_metal: bool,
    ) -> ExperienceTier {
        // Full experience: 8GB+ VRAM or Apple Silicon
        if vram_mb >= 8192 || has_metal {
            return ExperienceTier::Full;
        }

        // CPU-optimized: 16GB+ RAM
        if ram_mb >= 16384 {
            return ExperienceTier::CpuOptimized;
        }

        // Cloud fallback: 8GB+ RAM
        if ram_mb >= 8192 {
            return ExperienceTier::CloudFallback;
        }

        // Minimal: No AI features
        ExperienceTier::Minimal
    }

    /// Record milestone
    pub fn record_milestone(&mut self, name: &str) {
        let now = Utc::now();
        let duration_ms = (now - self.start_time).num_milliseconds() as u64;

        self.milestones.push(Milestone {
            name: name.to_string(),
            achieved_at: now,
            duration_ms,
        });

        // Check for wow moment
        if name == "first_ai_completion" && duration_ms < 30_000 {
            self.wow_moment_achieved = true;
        }
    }

    /// Transition state
    pub fn transition(&mut self, new_state: OnboardingState) {
        self.state = new_state.clone();
    }

    /// Get time-to-first-AI
    pub fn time_to_first_ai(&self) -> Option<u64> {
        self.milestones
            .iter()
            .find(|m| m.name == "first_ai_completion")
            .map(|m| m.duration_ms)
    }

    /// Check if onboarding is complete
    pub fn is_complete(&self) -> bool {
        self.state == OnboardingState::Completed
    }

    /// Get recommended settings based on tier
    pub fn get_recommended_settings(&self) -> RecommendedSettings {
        match self.experience_tier {
            ExperienceTier::Full => RecommendedSettings {
                model_size: "7B-13B".to_string(),
                context_length: 8192,
                gpu_layers: -1, // All
                enable_rag: true,
                enable_agents: true,
                cloud_fallback: false,
            },
            ExperienceTier::CpuOptimized => RecommendedSettings {
                model_size: "1B-3B".to_string(),
                context_length: 4096,
                gpu_layers: 0,
                enable_rag: true,
                enable_agents: true,
                cloud_fallback: false,
            },
            ExperienceTier::CloudFallback => RecommendedSettings {
                model_size: "cloud".to_string(),
                context_length: 4096,
                gpu_layers: 0,
                enable_rag: false,
                enable_agents: true,
                cloud_fallback: true,
            },
            ExperienceTier::Minimal => RecommendedSettings {
                model_size: "none".to_string(),
                context_length: 0,
                gpu_layers: 0,
                enable_rag: false,
                enable_agents: false,
                cloud_fallback: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedSettings {
    pub model_size: String,
    pub context_length: usize,
    pub gpu_layers: i32,
    pub enable_rag: bool,
    pub enable_agents: bool,
    pub cloud_fallback: bool,
}

impl Default for OnboardingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-config initializer
pub struct ZeroConfig {
    config_path: PathBuf,
    first_run: bool,
}

impl ZeroConfig {
    pub fn new() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kro_ide");

        let first_run = !config_path.exists();

        Self {
            config_path,
            first_run,
        }
    }

    /// Check if first run
    pub fn is_first_run(&self) -> bool {
        self.first_run
    }

    /// Initialize with sensible defaults
    pub fn initialize(&self) -> anyhow::Result<()> {
        if self.first_run {
            std::fs::create_dir_all(&self.config_path)?;

            // Create default config
            let config = serde_json::json!({
                "version": 1,
                "created_at": Utc::now().to_rfc3339(),
                "ai": {
                    "provider": "auto",
                    "model": "auto",
                    "context_length": "auto"
                },
                "ui": {
                    "theme": "dark",
                    "font_size": 14,
                    "minimap": true,
                    "sidebar": "left"
                },
                "editor": {
                    "tab_size": 4,
                    "word_wrap": true,
                    "auto_save": true,
                    "format_on_save": true
                }
            });

            std::fs::write(
                self.config_path.join("config.json"),
                serde_json::to_string_pretty(&config)?,
            )?;
        }

        Ok(())
    }
}

impl Default for ZeroConfig {
    fn default() -> Self {
        Self::new()
    }
}
