//! Load hardware profiles for AWS MCP bench campaigns.
//!
//! Default: in-repo `testdata/profiles.json`. Override with
//! `PHOTON_LEPTOS_BENCH_PROFILES` when pointing at an operator-managed file.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesFile {
    pub profiles: std::collections::HashMap<String, HardwareProfile>,
    #[serde(default)]
    pub amis: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub loadgen_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub phase: u32,
    pub wired: bool,
    pub instance_type: String,
    pub architecture: String,
    pub ami_key: String,
    pub server_role: bool,
    pub loadgen_role: bool,
    pub photon_hardware_tag: String,
    pub expected_vcpu: u32,
    pub expected_ram_gib: u32,
    pub ulimit_nofile: u32,
    pub ebs_gp3_iops: u32,
    #[serde(default)]
    pub tier: String,
}

pub fn profiles_path() -> PathBuf {
    std::env::var("PHOTON_LEPTOS_BENCH_PROFILES").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/profiles.json"),
        PathBuf::from,
    )
}

pub fn load_profiles() -> Result<ProfilesFile> {
    let path = profiles_path();
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read profiles at {}", path.display()))?;
    serde_json::from_str(&raw).context("parse profiles.json")
}

pub fn all_phase1_profiles() -> Result<Vec<String>> {
    let file = load_profiles()?;
    let mut ids: Vec<_> = file
        .profiles
        .iter()
        .filter(|(k, p)| p.phase == 1 && p.wired && k.starts_with("aws-"))
        .map(|(k, _)| k.clone())
        .collect();
    ids.sort();
    Ok(ids)
}

pub fn validate_hardware(profile: &str, allow_phase2: bool) -> Result<HardwareProfile> {
    let file = load_profiles()?;
    let p = file
        .profiles
        .get(profile)
        .with_context(|| format!("unknown hardware profile: {profile}"))?
        .clone();
    if p.phase == 2 && !p.wired && !allow_phase2 {
        bail!("profile {profile} is Phase 2 (not wired); pass --allow-phase2 to override");
    }
    if !p.wired && !allow_phase2 {
        bail!("profile {profile} is not wired for benchmarks");
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase1_profiles_non_empty() {
        let ids = all_phase1_profiles().expect("profiles.json");
        assert_eq!(ids.len(), 7, "{ids:?}");
    }
}
