//! Rebuild a new easyPDF staging tree from an exact published upstream release.
//! Only hash-verified, standalone product files and exact branding equivalents are copied.
//! Mixed hooks remain an explicit queue; this command never promotes a stage.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::{
    add_worktree, atomic_write, checked_relative, isolation, load_config, normalise_app_paths, normalize_staging_rust_identifiers,
    reconcile_staging_manifests, release_tag, rev_parse, sha256, validate_config,
};

mod ai_commands;
mod asset_manifest;
mod asset_renderer;
mod chrome_width;
mod ci_brand;
mod ci_pin;
mod cli_brand;
mod connected_ui;
mod cover;
mod cover_translations;
mod cover_ui;
mod crate_readmes_brand;
mod demo_brand;
mod demo_xtask_brand;
mod desktop_brand;
mod dmg_simple;
mod docker_pin;
mod dockerignore_stage;
mod docs_brand;
mod final_path_audit;
mod font_pin;
mod format_stage;
mod headless_bridge;
mod headless_security;
mod home_screenshots;
mod i18n_build;
mod instructions_brand;
mod layers_brand;
mod links_brand;
mod macos_brand;
mod metadata_brand;
mod nextest_brand;
mod notice_brand;
mod ocr_models_brand;
mod packaging_env;
mod parity_paths;
mod pdf_markers;
mod platform_icons;
mod portal_brand_assets;
mod readme_brand;
mod release_docs;
mod release_guard;
mod runtime_brand;
mod runtime_brand_tail;
mod runtime_strings;
mod sign_brand_test;
mod site_navigation;
mod tool_order;
mod trunk_cache;
mod ui_brand;
mod ui_id_compat;
mod ui_test_patch;
mod unix_packages;
mod unsigned_update_guard;
mod version_align;
mod wasm_portal;
mod web_package;
mod windows_msi;
mod xfa_wrap;

#[derive(Deserialize)]
struct Inventory {
    release: String,
    upstream_sha: String,
    entries: Vec<InventoryEntry>,
    components: Vec<Component>,
    brand_asset_replacements: Vec<BrandAsset>,
}

#[derive(Deserialize)]
struct Component {
    hook_paths: Vec<String>,
}

#[derive(Deserialize)]
struct InventoryEntry {
    path: String,
    classification: String,
    upstream_sha256: Option<String>,
    easypdf_sha256: Option<String>,
}

#[derive(Deserialize)]
struct BrandAsset {
    replacement_path: String,
    valid: bool,
}

#[derive(Deserialize)]
struct OwnedManifest {
    schema: u32,
    files: Vec<OwnedFile>,
}

#[derive(Deserialize)]
struct OwnedFile {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct FreshReport {
    schema: u32,
    release: String,
    upstream_sha: String,
    stage: String,
    owned_files: usize,
    brand_only_files: usize,
    brand_icons: usize,
    brand_package_files: usize,
    connected_web_host_applied: bool,
    ui_branding_applied: bool,
    font_policy_guard_applied: bool,
    cover_engine_and_tool_applied: bool,
    cover_ui_base_applied: bool,
    connected_ui_seam_applied: bool,
    tool_order_port_replayable: bool,
    update_channel_disabled: bool,
    mixed_hooks: Vec<String>,
    feature_hooks: Vec<String>,
    integrated_hooks: Vec<String>,
    missing_required_package_files: Vec<String>,
    status: &'static str,
}

pub(super) fn brand_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "desktop-brand")?;
    desktop_brand::apply(root, &stage, &backup_release)
}

pub(super) fn brand_paths_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "desktop-paths")?;
    desktop_brand::apply_paths(root, &stage, &backup_release)
}

pub(super) fn brand_ui_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "ui-brand")?;
    let count = ui_brand::apply(root, &stage, &backup_release)?;
    println!("Rebranded {count} UI source and translation files in {}", stage.display());
    Ok(())
}

pub(super) fn brand_cli_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "cli-brand")?;
    cli_brand::apply(root, &stage, &backup_release)
}

pub(super) fn brand_links_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "links-brand")?;
    let release = args.first().context("missing reviewed release")?;
    links_brand::apply(root, &stage, release, &backup_release)
}

pub(super) fn dmg_simple_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "dmg-simple")?;
    let release = args.first().context("missing reviewed release")?;
    dmg_simple::apply(root, &stage, release, &backup_release)
}

pub(super) fn dmg_attribution_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "dmg-attribution")?;
    let release = args.first().context("missing reviewed release")?;
    dmg_simple::apply_attribution(root, &stage, release, &backup_release)
}

pub(super) fn asset_renderer_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "asset-renderer")?;
    let release = args.first().context("missing reviewed release")?;
    asset_renderer::apply(root, &stage, release, &backup_release)
}

pub(super) fn macos_brand_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "macos-brand")?;
    let release = args.first().context("missing reviewed release")?;
    macos_brand::apply(root, &stage, release, &backup_release)
}

pub(super) fn packaging_env_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "packaging-env")?;
    let release = args.first().context("missing reviewed release")?;
    packaging_env::apply(root, &stage, release, &backup_release)
}

pub(super) fn windows_msi_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "windows-msi")?;
    let release = args.first().context("missing reviewed release")?;
    windows_msi::apply(root, &stage, release, &backup_release)
}

pub(super) fn asset_manifest_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "asset-manifest")?;
    let release = args.first().context("missing reviewed release")?;
    asset_manifest::apply(root, &stage, release, &backup_release)
}

pub(super) fn unix_packages_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "unix-packages")?;
    let release = args.first().context("missing reviewed release")?;
    unix_packages::apply(root, &stage, release, &backup_release)
}

pub(super) fn web_package_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "web-package")?;
    let release = args.first().context("missing reviewed release")?;
    web_package::apply(root, &stage, release, &backup_release)
}

pub(super) fn runtime_brand_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "runtime-brand")?;
    let release = args.first().context("missing reviewed release")?;
    runtime_brand::apply(root, &stage, release, &backup_release)
}

pub(super) fn normalize_manifests_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, _) = existing_stage(root, args, "normalize-manifests")?;
    reconcile_staging_manifests(root, &stage)?;
    Ok(())
}

pub(super) fn release_guard_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "release-guard")?;
    let release = args.first().context("missing reviewed release")?;
    release_guard::apply(root, &stage, release, &backup_release)
}

pub(super) fn headless_bridge_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "headless-bridge")?;
    let release = args.first().context("missing reviewed release")?;
    headless_bridge::apply(root, &stage, release, &backup_release)
}

pub(super) fn headless_security_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "headless-security")?;
    let release = args.first().context("missing reviewed release")?;
    headless_security::apply(root, &stage, release, &backup_release)
}

pub(super) fn parity_paths_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "parity-paths")?;
    let release = args.first().context("missing reviewed release")?;
    parity_paths::apply(root, &stage, release, &backup_release)
}

pub(super) fn layers_brand_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "layers-brand")?;
    let release = args.first().context("missing reviewed release")?;
    layers_brand::apply(root, &stage, release, &backup_release)
}

pub(super) fn runtime_strings_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "runtime-strings")?;
    let release = args.first().context("missing reviewed release")?;
    runtime_strings::apply(root, &stage, release, &backup_release)
}

pub(super) fn platform_icons_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "platform-icons")?;
    let release = args.first().context("missing reviewed release")?;
    platform_icons::apply(root, &stage, release, &backup_release)
}

pub(super) fn font_pin_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "font-pin")?;
    font_pin::apply(root, &stage, &backup_release)
}

pub(super) fn portal_brand_assets_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "portal-brand-assets")?;
    let release = args.first().context("missing reviewed release")?;
    portal_brand_assets::apply(root, &stage, release, &backup_release)
}

pub(super) fn cover_translations_existing(root: &Path, args: &[String]) -> Result<()> {
    let (stage, backup_release) = existing_stage(root, args, "cover-translations")?;
    cover_translations::apply(root, &stage, &backup_release)
}

/// Resume one fail-closed tail adapter on an incomplete v0.19 replay. This is
/// for adapting changed release inputs without rebuilding the worktree after
/// each rejected hash; the normal `fresh` pipeline remains the final proof.
pub(super) fn replay_tail_step_existing(root: &Path, args: &[String]) -> Result<()> {
    let [release, step, flag] = args else {
        bail!("replay-tail-step requires v0.19.0 <step> --replay-check");
    };
    if release != "v0.19.0" || flag != "--replay-check" {
        bail!("replay-tail-step is limited to the incomplete v0.19.0 replay-check tree");
    }
    let (stage, backup_label) = existing_stage(root, &[release.clone(), flag.clone()], "replay-tail-step")?;
    match step.as_str() {
        "home-screenshots" => home_screenshots::apply(root, &stage, release, &backup_label),
        "site-navigation" => site_navigation::apply(root, &stage, &backup_label),
        "readme-brand" => readme_brand::apply(root, &stage, release, &backup_label),
        "metadata-brand" => metadata_brand::apply(root, &stage, &backup_label),
        "ci-brand" => ci_brand::apply(root, &stage, &backup_label),
        "instructions-brand" => instructions_brand::apply(root, &stage, release, &backup_label),
        "crate-readmes-brand" => crate_readmes_brand::apply(root, &stage, &backup_label),
        "unsigned-update-guard" => unsigned_update_guard::apply(root, &stage, release, &backup_label),
        "ui-id-compat" => ui_id_compat::apply(root, &stage, &backup_label),
        "docker-pin" => docker_pin::apply(root, &stage, &backup_label),
        "dockerignore-stage" => dockerignore_stage::apply(root, &stage, release, &backup_label),
        "trunk-cache" => trunk_cache::apply(root, &stage, release, &backup_label),
        "format-stage" => format_stage::apply(root, &stage, &backup_label),
        "chrome-width" => chrome_width::apply(root, &stage, release, &backup_label),
        "ui-test-patch" => ui_test_patch::apply(root, &stage, release, &backup_label),
        "runtime-brand-tail" => runtime_brand_tail::apply(root, &stage, release, &backup_label),
        "pdf-markers" => pdf_markers::apply(root, &stage, release, &backup_label),
        "xfa-wrap" => xfa_wrap::apply(root, &stage, release, &backup_label),
        "sign-brand-test" => sign_brand_test::apply(root, &stage, release, &backup_label),
        "version-align" => version_align::apply(root, &stage, release, &backup_label),
        _ => bail!("unknown v0.19 replay tail step: {step}"),
    }
}

fn existing_stage(root: &Path, args: &[String], command: &str) -> Result<(PathBuf, String)> {
    let (release, replay_check) = match args {
        [release] => (release, false),
        [release, flag] if flag == "--replay-check" => (release, true),
        _ => bail!("{command} requires a release tag: {command} v0.19.0 [--replay-check]"),
    };
    release_tag(release)?;
    let config = load_config(root)?;
    validate_config(&config)?;
    if release != &config.release_ref {
        bail!("{command} release `{release}` is not the owner-verified release `{}`", config.release_ref);
    }
    let stage_name = if replay_check { format!("{release}-easypdf-replay-check") } else { format!("{release}-easypdf-fresh") };
    let stage = root.join("upstream/staging").join(stage_name);
    if !std::fs::symlink_metadata(&stage).context("inspect incomplete stage")?.file_type().is_dir()
        || !stage.join("EASYPDF-STAGING-INCOMPLETE").is_file()
    {
        bail!("{command} only accepts an incomplete, real staging directory: {}", stage.display());
    }
    let backup_release = if replay_check { format!("{release}-replay-check") } else { release.clone() };
    Ok((stage, backup_release))
}

pub(super) fn run(root: &Path, args: &[String]) -> Result<()> {
    let (release, verify_replay) = match args {
        [release] => (release, false),
        [release, flag] if flag == "--verify-replay" => (release, true),
        _ => bail!("fresh requires a published release tag: fresh v0.19.0 [--verify-replay]"),
    };
    release_tag(release)?;
    let config = load_config(root)?;
    validate_config(&config)?;
    if release != &config.release_ref {
        bail!("fresh release `{release}` is not the owner-verified release `{}`", config.release_ref);
    }
    // Only audited release inputs may enter this staged replay.
    // Every adapter still verifies its own exact inputs before changing them.
    if !matches!(release.as_str(), "v0.18.0" | "v0.18.1" | "v0.18.2" | "v0.19.0") {
        bail!("fresh replay for `{release}` is not yet reviewed; no staging tree was created");
    }
    let cache = root.join("target/upstream-cache/pdfcraft.git");
    let upstream_sha = rev_parse(&cache, &format!("refs/tags/{release}^{{commit}}"))?;
    let stage_name = if verify_replay { format!("{release}-easypdf-replay-check") } else { format!("{release}-easypdf-fresh") };
    let stage = root.join("upstream/staging").join(&stage_name);
    // Each fresh run gets its own safeguard tree. A failed earlier replay must never make a
    // subsequent clean run reuse or overwrite its pre-apply copies.
    let backup_release = if release == "v0.19.0" {
        let run = SystemTime::now().duration_since(UNIX_EPOCH).context("read backup run timestamp")?.as_nanos();
        format!("{release}-fresh-run-{run}")
    } else if verify_replay {
        format!("{release}-replay-check")
    } else {
        release.clone()
    };
    if stage.exists() {
        bail!("fresh stage already exists; refuse to overwrite {}", stage.display());
    }

    // Rebuild and revalidate the input layer on every invocation, so old snapshots cannot be
    // silently replayed after the product tree or the upstream tag changes.
    isolation::audit(root, std::slice::from_ref(release))?;
    let inventory_path = root.join("upstream/isolation").join(format!("{release}-inventory.json"));
    let inventory: Inventory =
        serde_json::from_slice(&std::fs::read(&inventory_path).context("read fresh inventory")?).context("parse fresh inventory")?;
    if inventory.release != *release || inventory.upstream_sha != upstream_sha {
        bail!("fresh inventory does not match the pinned release");
    }
    let manifest: OwnedManifest =
        serde_json::from_slice(&std::fs::read(root.join("upstream/isolation/owned-manifest.json")).context("read owned layer manifest")?)
            .context("parse owned layer manifest")?;
    if manifest.schema != 1 || manifest.files.len() > 500 {
        bail!("unsupported or oversized owned layer manifest");
    }

    add_worktree(&cache, &stage, &upstream_sha)?;
    atomic_write(&stage.join("EASYPDF-STAGING-INCOMPLETE"), b"Fresh easyPDF layer application is incomplete. Do not run or promote this tree.\n")?;
    tool_order::apply(root, &stage, release, &backup_release)?;
    normalise_app_paths(&stage)?;
    if matches!(release.as_str(), "v0.18.1" | "v0.18.2" | "v0.19.0") {
        i18n_build::apply(root, &stage, &backup_release)?;
    }
    let brand_only_files = apply_brand_equivalents(root, &stage, &inventory)?;
    let owned_files = apply_owned_files(root, &stage, &manifest)?;
    if matches!(release.as_str(), "v0.18.1" | "v0.18.2" | "v0.19.0") {
        i18n_build::configure_docker(root, &stage, &backup_release)?;
        web_package::configure_staging_port(root, &stage, release, &backup_release)?;
    }
    let brand_icons = apply_brand_icons(root, &stage, &inventory, &backup_release)?;
    let brand_package_files = apply_brand_package_files(root, &stage)?;
    let policy = std::fs::read_to_string(root.join("AGENTS.md")).context("read easyPDF repository policy")?;
    let policy = format!("{}\n", policy.replace("\r\n", "\n").trim_end_matches(['\r', '\n']));
    atomic_write(&stage.join("AGENTS.md"), policy.as_bytes())?;
    reconcile_staging_manifests(root, &stage)?;
    normalize_staging_rust_identifiers(&stage)?;
    apply_web_host(root, &stage, release, &backup_release)?;
    apply_ui_branding(&stage)?;
    apply_visible_branding(&stage)?;
    apply_font_policy(&stage)?;
    cover::apply(root, &stage, &backup_release)?;
    cover_ui::apply(root, &stage, &backup_release)?;
    disable_unapproved_update_channel(&stage)?;
    connected_ui::apply(root, &stage, release, &backup_release)?;
    tool_order::apply_home(root, &stage)?;
    desktop_brand::apply(root, &stage, &backup_release)?;
    desktop_brand::apply_paths(root, &stage, &backup_release)?;
    desktop_brand::apply_tests(root, &stage, &backup_release)?;
    desktop_brand::apply_metadata(root, &stage, &backup_release)?;
    ui_brand::apply(root, &stage, &backup_release)?;
    cli_brand::apply(root, &stage, &backup_release)?;
    links_brand::apply(root, &stage, release, &backup_release)?;
    asset_renderer::apply(root, &stage, release, &backup_release)?;
    dmg_simple::apply(root, &stage, release, &backup_release)?;
    macos_brand::apply(root, &stage, release, &backup_release)?;
    packaging_env::apply(root, &stage, release, &backup_release)?;
    asset_manifest::apply(root, &stage, release, &backup_release)?;
    demo_brand::apply(root, &stage, &backup_release)?;
    if matches!(release.as_str(), "v0.18.2" | "v0.19.0") {
        demo_xtask_brand::apply(root, &stage, &backup_release)?;
    }
    windows_msi::apply(root, &stage, release, &backup_release)?;
    unix_packages::apply(root, &stage, release, &backup_release)?;
    web_package::apply(root, &stage, release, &backup_release)?;
    runtime_brand::apply(root, &stage, release, &backup_release)?;
    release_guard::apply(root, &stage, release, &backup_release)?;
    release_docs::apply(root, &stage, release, &backup_release)?;
    docs_brand::apply(root, &stage, release, &backup_release)?;
    if matches!(release.as_str(), "v0.18.1" | "v0.18.2" | "v0.19.0") {
        headless_bridge::apply(root, &stage, release, &backup_release)?;
        headless_security::apply(root, &stage, release, &backup_release)?;
        parity_paths::apply(root, &stage, release, &backup_release)?;
        layers_brand::apply(root, &stage, release, &backup_release)?;
        runtime_strings::apply(root, &stage, release, &backup_release)?;
        platform_icons::apply(root, &stage, release, &backup_release)?;
        font_pin::apply(root, &stage, &backup_release)?;
        portal_brand_assets::apply(root, &stage, release, &backup_release)?;
        cover_translations::apply(root, &stage, &backup_release)?;
        home_screenshots::apply(root, &stage, release, &backup_release)?;
        site_navigation::apply(root, &stage, &backup_release)?;
        readme_brand::apply(root, &stage, release, &backup_release)?;
        notice_brand::apply(root, &stage, release, &backup_release)?;
        metadata_brand::apply(root, &stage, &backup_release)?;
        nextest_brand::apply(root, &stage, release, &backup_release)?;
        ci_brand::apply(root, &stage, &backup_release)?;
        ci_pin::apply(root, &stage, release, &backup_release)?;
        instructions_brand::apply(root, &stage, release, &backup_release)?;
        crate_readmes_brand::apply(root, &stage, &backup_release)?;
        unsigned_update_guard::apply(root, &stage, release, &backup_release)?;
        ui_id_compat::apply(root, &stage, &backup_release)?;
        docker_pin::apply(root, &stage, &backup_release)?;
        dockerignore_stage::apply(root, &stage, release, &backup_release)?;
        trunk_cache::apply(root, &stage, release, &backup_release)?;
        format_stage::apply(root, &stage, &backup_release)?;
        chrome_width::apply(root, &stage, release, &backup_release)?;
        ui_test_patch::apply(root, &stage, release, &backup_release)?;
        runtime_brand_tail::apply(root, &stage, release, &backup_release)?;
        pdf_markers::apply(root, &stage, release, &backup_release)?;
        xfa_wrap::apply(root, &stage, release, &backup_release)?;
        sign_brand_test::apply(root, &stage, release, &backup_release)?;
        version_align::apply(root, &stage, release, &backup_release)?;
        if release == "v0.19.0" {
            ai_commands::apply(root, &stage, &backup_release)?;
            ocr_models_brand::apply(root, &stage, &backup_release)?;
            wasm_portal::apply(root, &stage, &backup_release)?;
            final_path_audit::apply(root, &stage, &backup_release)?;
        }
    }

    let mut mixed_hooks =
        inventory.entries.iter().filter(|entry| entry.classification == "mixed-manual-port").map(|entry| entry.path.clone()).collect::<Vec<_>>();
    mixed_hooks.sort();
    let feature_hooks = inventory
        .components
        .iter()
        .flat_map(|component| component.hook_paths.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let integrated_hooks = vec!["apps/easypdf-web/src/main.rs".to_string()];
    if !integrated_hooks.iter().all(|path| feature_hooks.contains(path)) {
        bail!("a replayed fresh-layer hook is missing from the audited feature-hook inventory");
    }
    let metadata = Command::new("cargo")
        .current_dir(&stage)
        .args(["metadata", "--no-deps", "--offline", "--format-version", "1"])
        .output()
        .context("check fresh staging Cargo manifests")?;
    if !metadata.status.success() {
        bail!("fresh stage manifests are not valid: {}", String::from_utf8_lossy(&metadata.stderr).trim());
    }

    // A stage may be useful for source review without being packageable. In
    // particular, the original Windows MSI is removed by the branding policy;
    // the easyPDF replacement must be explicitly ported, not silently assumed.
    let missing_required_package_files =
        ["packaging/windows/easypdf.wxs"].into_iter().filter(|path| !stage.join(path).is_file()).map(str::to_owned).collect::<Vec<_>>();

    let report = FreshReport {
        schema: 1,
        release: release.clone(),
        upstream_sha,
        stage: format!("upstream/staging/{stage_name}"),
        owned_files,
        brand_only_files,
        brand_icons,
        brand_package_files,
        connected_web_host_applied: true,
        ui_branding_applied: true,
        font_policy_guard_applied: true,
        cover_engine_and_tool_applied: true,
        cover_ui_base_applied: true,
        connected_ui_seam_applied: true,
        tool_order_port_replayable: verify_replay,
        update_channel_disabled: true,
        mixed_hooks,
        feature_hooks,
        integrated_hooks,
        missing_required_package_files,
        status: "not-ready: remaining feature hooks, mixed-path review, package files, complete branding, full asset audit and web tests remain",
    };
    let report_name = if verify_replay { format!("{release}-replay-check-report.json") } else { format!("{release}-fresh-report.json") };
    let output = root.join("upstream/isolation").join(&report_name);
    atomic_write(&output, &serde_json::to_vec_pretty(&report).context("serialize fresh staging report")?)?;
    let product_version = if release == "v0.19.0" { "v0.1.0 beta" } else { release.as_str() };
    atomic_write(
        &stage.join("EASYPDF-STAGING-STATUS.md"),
        format!(
            "# easyPDF {product_version} fresh stage\n\nUpstream release: {release}, commit `{}`.\n\nApplied {} standalone files, {} exact branding equivalents, {} verified trademark icons and {} easyPDF package descriptors. The connected web host retains the upstream same-origin save fix. The Apply Covers engine, headless tool and editor dialog, connected account/kit/AI workspace, and tool-order layer have replay scripts. The unapproved update channel is disabled; the upstream contributor panel and linked artwork are removed; optional Adobe-origin font families are excluded.\n\n**Not ready for use or promotion:** {} declared feature-hook paths have no complete integration proof; {} mixed paths require review (not all need porting); {} required package files are missing. Full fresh-tree replay, branding, asset audit, desktop/web interaction and end-to-end gates remain. Keep `EASYPDF-STAGING-INCOMPLETE` until all gates pass.\n\nMachine report: `upstream/isolation/{report_name}`.\n",
            report.upstream_sha,
            report.owned_files,
            report.brand_only_files,
            report.brand_icons,
            report.brand_package_files,
            report.feature_hooks.len().saturating_sub(report.integrated_hooks.len()),
            report.mixed_hooks.len(),
            report.missing_required_package_files.len()
        )
        .as_bytes(),
    )?;
    println!(
        "Fresh {release} stage: {} owned, {} exact branding, {} icons, {} package descriptors; {} feature hooks, {} mixed paths and {} missing package files to review; not ready for promotion",
        owned_files,
        brand_only_files,
        brand_icons,
        brand_package_files,
        report.feature_hooks.len().saturating_sub(report.integrated_hooks.len()),
        report.mixed_hooks.len(),
        report.missing_required_package_files.len()
    );
    println!("Stage: {}", stage.display());
    Ok(())
}

fn apply_brand_equivalents(root: &Path, stage: &Path, inventory: &Inventory) -> Result<usize> {
    let mut count = 0;
    for entry in inventory.entries.iter().filter(|entry| entry.classification == "branding-candidate") {
        let relative = checked_relative(&entry.path)?;
        let upstream_hash = entry.upstream_sha256.as_deref().context("branding candidate lacks upstream hash")?;
        let local_hash = entry.easypdf_sha256.as_deref().context("branding candidate lacks easyPDF hash")?;
        let source = read_bounded(&root.join(&relative))?;
        let existing = read_bounded(&stage.join(&relative))?;
        if sha256(&source) != local_hash || sha256(&existing) != upstream_hash {
            bail!("branding candidate changed after audit: {}", entry.path);
        }
        atomic_write(&stage.join(relative), &source)?;
        count += 1;
    }
    Ok(count)
}

fn apply_owned_files(root: &Path, stage: &Path, manifest: &OwnedManifest) -> Result<usize> {
    let mut count = 0;
    for file in &manifest.files {
        let relative = checked_relative(&file.path)?;
        let source = read_bounded(&root.join("upstream/isolation/owned").join(&relative))?;
        if sha256(&source) != file.sha256 || sha256(&read_bounded(&root.join(&relative))?) != file.sha256 {
            bail!("owned easyPDF file changed after audit: {}", file.path);
        }
        let destination = stage.join(&relative);
        if destination.exists() {
            bail!("owned easyPDF path now exists upstream: {}", file.path);
        }
        atomic_write(&destination, &source)?;
        count += 1;
    }
    Ok(count)
}

fn apply_brand_icons(root: &Path, stage: &Path, inventory: &Inventory, backup_label: &str) -> Result<usize> {
    let manifest_text = std::fs::read_to_string(root.join("ATTRIBUTION.toml")).context("read easyPDF asset manifest")?;
    let manifest: toml::Value = toml::from_str(&manifest_text).context("parse easyPDF asset manifest")?;
    let mut hashes = BTreeMap::new();
    for entry in manifest.get("asset").and_then(toml::Value::as_array).context("asset manifest has no assets")? {
        if let (Some(path), Some(hash), Some(licence)) = (
            entry.get("path").and_then(toml::Value::as_str),
            entry.get("sha256").and_then(toml::Value::as_str),
            entry.get("licence").and_then(toml::Value::as_str),
        ) && licence == "LicenseRef-Twinba-Trademark"
        {
            hashes.insert(path, hash);
        }
    }
    let mut count = 0;
    for icon in &inventory.brand_asset_replacements {
        if !icon.valid || !icon.replacement_path.starts_with("assets/app-icon/") {
            bail!("unverified easyPDF brand icon: {}", icon.replacement_path);
        }
        let relative = checked_relative(&icon.replacement_path)?;
        let expected = hashes.get(icon.replacement_path.as_str()).context("brand icon is absent from ATTRIBUTION.toml")?;
        let bytes = read_bounded(&root.join(&relative))?;
        if sha256(&bytes) != **expected {
            bail!("brand icon hash changed after audit: {}", icon.replacement_path);
        }
        let destination = stage.join(&relative);
        if destination.exists() && read_bounded(&destination)? != bytes {
            bail!("upstream now owns the easyPDF icon path: {}", icon.replacement_path);
        }
        atomic_write(&destination, &bytes)?;
        count += 1;
    }
    if matches!(inventory.release.as_str(), "v0.18.2" | "v0.19.0") {
        // Upstream's README still describes its removed artwork. Preserve the exact old
        // text as a backup and install the reviewed easyPDF icon documentation instead.
        let relative = "assets/app-icon/README.md";
        let old = read_bounded(&stage.join(relative))?;
        let owner = read_bounded(&root.join(relative))?;
        if !sha256(&old).eq_ignore_ascii_case("B8876253A447E09F192F3739E30CF1AE86D8353B8C0DBFF83167CF411850A1B2")
            || !sha256(&owner).eq_ignore_ascii_case("B68EB00AD199A4D9401201ED5F38D9D796C70DD4D4F88521464F3474AA180E2E")
        {
            bail!("icon README changed; review its branding before replay");
        }
        let label = if inventory.release == "v0.19.0" { backup_label } else { &inventory.release };
        let backup = root.join("upstream/backups").join(format!("{label}-fresh-icon-readme-before-apply")).join(relative);
        if backup.exists() {
            bail!("icon README backup already exists: {}", backup.display());
        }
        atomic_write(&backup, &old)?;
        atomic_write(&stage.join(relative), &owner)?;
        count += 1;
    }
    Ok(count)
}

fn apply_brand_package_files(root: &Path, stage: &Path) -> Result<usize> {
    // These are easyPDF's own app-ID descriptors. The upstream PdfCraft variants were removed
    // by `normalise_app_paths`; do not copy broad packaging trees over newer release logic.
    const FILES: &[&str] = &[
        "packaging/linux/it.twinba.easypdf.desktop",
        "packaging/linux/it.twinba.easypdf.metainfo.xml.in",
        "packaging/linux/it.twinba.easypdf.mime.xml",
        "packaging/linux/flatpak/it.twinba.easypdf.bundle.yml",
        "packaging/linux/flatpak/it.twinba.easypdf.yml",
    ];
    for path in FILES {
        let relative = checked_relative(path)?;
        let bytes = read_bounded(&root.join(&relative))?;
        let destination = stage.join(&relative);
        if destination.exists() && read_bounded(&destination)? != bytes {
            bail!("upstream now owns the easyPDF package path: {path}");
        }
        atomic_write(&destination, &bytes)?;
    }
    Ok(FILES.len())
}

fn apply_font_policy(stage: &Path) -> Result<()> {
    // Keep new upstream font-size/script handling, but never package Adobe-origin designs.
    // Exact anchors force review if a future release reorganises this build path.
    let scripts = stage.join("crates/fonts/src/cjk_scripts.rs");
    replace_exact_once(
        &scripts,
        "pub const WHOLE_CJK_FAMILIES: [&str; 3] = [\"Shippori Mincho\", \"BIZ UDPGothic\", \"BIZ UDMincho\"];",
        "pub const WHOLE_CJK_FAMILIES: [&str; 3] = [\"Shippori Mincho\", \"BIZ UDPGothic\", \"BIZ UDMincho\"];\n\n/// Adobe-origin type designs forbidden by AGENTS.md, regardless of size or licence.\npub const BARRED_FAMILIES: [&str; 5] = [\"Source Han\", \"Source Serif\", \"Source Sans\", \"Noto Sans CJK\", \"Noto Serif CJK\"];\n\n/// A prefix also covers region and style suffixes in the optional font manifest.\npub fn is_barred_family(family: &str) -> bool {\n    BARRED_FAMILIES.iter().any(|prefix| family.starts_with(prefix))\n}",
    )?;
    replace_exact_once(
        &scripts,
        "pub fn cjk_size_decision(family: &str, bytes: u64, wasm: bool, required: bool) -> CjkSizeDecision {\n    if WHOLE_CJK_FAMILIES.contains(&family) {",
        "pub fn cjk_size_decision(family: &str, bytes: u64, wasm: bool, required: bool) -> CjkSizeDecision {\n    if is_barred_family(family) {\n        return CjkSizeDecision::Skip;\n    }\n    if WHOLE_CJK_FAMILIES.contains(&family) {",
    )?;
    replace_exact_once(
        &scripts,
        "mod tests {\n    use super::*;\n",
        "mod tests {\n    use super::*;\n\n    #[test]\n    fn adobe_origin_families_never_pass_the_font_size_gate() {\n        for family in [\"Source Han Sans\", \"Source Serif 4\", \"Source Sans 3\", \"Noto Sans CJK SC\", \"Noto Serif CJK JP\"] {\n            assert_eq!(cjk_size_decision(family, 1024, false, false), CjkSizeDecision::Skip, \"{family}\");\n        }\n        assert!(!is_barred_family(\"BIZ UDPGothic\"));\n        assert!(!is_barred_family(\"LXGW WenKai Lite\"));\n    }\n",
    )?;
    let build = stage.join("crates/fonts/build.rs");
    replace_exact_once(
        &build,
        "        let [family, style, file, scripts, ..] = f.as_slice() else {\n            return Err(format!(\"malformed manifest line: {line}\"));\n        };",
        "        let [family, style, file, scripts, ..] = f.as_slice() else {\n            return Err(format!(\"malformed manifest line: {line}\"));\n        };\n        if cjk_scripts::is_barred_family(family) {\n            println!(\"cargo::warning=craft-fonts: not embedding {family} {style} (AGENTS.md section 1.1)\");\n            continue;\n        }",
    )?;
    Ok(())
}

fn apply_ui_branding(stage: &Path) -> Result<()> {
    // Remove the upstream contributor UI and ArtCraft artwork without touching the
    // required third-party notices. Range hashes pin the exact reviewed v0.17 bodies.
    let ui = stage.join("crates/ui-egui/src");
    replace_exact_once(&ui.join("lib.rs"), "use dialogs::credits;\n", "")?;
    replace_exact_once(&ui.join("dialogs.rs"), "pub(super) mod credits;\n", "")?;
    replace_exact_once(
        &ui.join("dialogs.rs"),
        "//! Modal dialogs: Document Properties, Keyboard Shortcuts, About (with the Contributors and Models credits).",
        "//! Modal dialogs: Document Properties, Keyboard Shortcuts, About.",
    )?;
    replace_exact_once(
        &ui.join("i18n/tests.rs"),
        "    crate::credits::NameMode::ALL.iter().for_each(|mode| add(mode.label()));\n    for key in crate::credits::SortKey::ALL {\n        let (label, header) = key.label();\n        add(label);\n        add(header);\n    }\n    crate::credits::MODEL_COLUMNS.iter().for_each(|s| add(s));\n",
        "",
    )?;
    replace_range_pinned(
        &ui.join("dialogs.rs"),
        "fn dialog_about(",
        "fn dialog_pin_tool(",
        "fef6e2e3c834cef99a815f4c1c9d10d656f8162dac7fcbe0c44a394aee8e791f",
        r#"fn dialog_about(ui: &mut egui::Ui, _req: &mut Pending) -> bool {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        widgets::brand_mark(ui, 40.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("easyPDF by Massimo Bertolucci").font(theme::semibold(20.0)));
            ui.label(crate::i18n::fmt(tl!("Version {v}"), &[("v", &easypdf_engine::build_info::version())]));
        });
    });
    ui.add_space(6.0);
    ui.label(tl!("A clean-room, open-source PDF application written in Rust. MIT OR Apache-2.0."));
    ui.label(
        egui::RichText::new("Rendering: hayro · UI: egui · Icons: Lucide (ISC) · Fonts: Inter, JetBrains Mono, Dancing Script (OFL)")
            .color(t.text_muted)
            .small(),
    );
    true
}

"#,
    )?;
    replace_range_pinned(
        &ui.join("home.rs"),
        "            if !app.community_card_closed {",
        "\n            egui::Frame::NONE\n",
        "5f05ea2f6d05f7bb318ea1168aae749d197c7ebfc588dc09e0b54ac769a18958",
        "",
    )?;
    replace_range_pinned(
        &ui.join("widgets.rs"),
        "/// The ArtCraft wordmark",
        "/// [`icon_pill`]",
        "ad10f917eb0ed91b5e8a89505d53c7876ca4dbcb8bdac55d96a5abfaf4c43f61",
        r#"/// The attributed easyPDF trademark icon, sized to `size` points square.
pub fn brand_mark(ui: &mut egui::Ui, size: f32) -> Response {
    ui.add(
        egui::Image::from_bytes("bytes://easypdf-icon.png", include_bytes!("../../../assets/app-icon/hicolor/128x128/apps/it.twinba.easypdf.png"))
            .fit_to_exact_size(vec2(size, size))
            .alt_text("easyPDF"),
    )
}

"#,
    )?;
    let credits = ui.join("dialogs/credits.rs");
    std::fs::remove_file(&credits).with_context(|| format!("remove upstream contributor UI {}", credits.display()))?;
    Ok(())
}

fn apply_visible_branding(stage: &Path) -> Result<()> {
    let ui = stage.join("crates/ui-egui/src");
    replace_exact_once(
        &ui.join("app.rs"),
        ".map_or_else(|| \"PdfCraft\".to_owned(), |name| format!(\"{name}{mark} — PdfCraft\"))",
        ".map_or_else(|| \"easyPDF\".to_owned(), |name| format!(\"{name}{mark} — easyPDF\"))",
    )?;
    replace_exact_once(&ui.join("app.rs"), "storage.set_string(\"pdfcraft\", self.persist());", "storage.set_string(\"easypdf\", self.persist());")?;
    replace_exact_once(&ui.join("menus.rs"), "pub(crate) const APP_NAME: &str = \"PdfCraft\";", "pub(crate) const APP_NAME: &str = \"easyPDF\";")?;
    replace_exact_once(&ui.join("home.rs"), "tl!(\"Welcome to PdfCraft\")", "tl!(\"Welcome to easyPDF\")")?;
    replace_exact_once(
        &ui.join("home.rs"),
        "tl!(\"Files you open in PdfCraft appear here. Drop a PDF anywhere to open it.\")",
        "tl!(\"Files you open in easyPDF appear here. Drop a PDF anywhere to open it.\")",
    )?;
    replace_exact_once(
        &ui.join("home.rs"),
        "tl!(\"PdfCraft works offline. No telemetry, no account, and no cloud processing unless you add a provider.\")",
        "tl!(\"easyPDF processes local files on your device. The protected web edition uses your authorized account.\")",
    )?;
    let desktop = stage.join("apps/easypdf/src/main.rs");
    replace_exact_once(&desktop, "const LEGACY_STORAGE_KEY: &str = \"printcraft\";\n", "")?;
    replace_exact_once(&desktop, "s.get_string(\"pdfcraft\").or_else(|| s.get_string(LEGACY_STORAGE_KEY))", "s.get_string(\"easypdf\")")?;
    for (old, new) in [
        ("//! PdfCraft desktop app.", "//! easyPDF desktop app."),
        ("//! Usage: `pdfcraft [options] [files…]`", "//! Usage: `easypdf [options] [files…]`"),
        ("//! hands them to the PdfCraft already running", "//! hands them to the easyPDF already running"),
        ("//! Agents then drive it with `pdfcraft-cli ui", "//! Agents then drive it with `easypdf-cli ui"),
        ("Scripts want `pdfcraft-cli`", "Scripts want `easypdf-cli`"),
        ("Usage: pdfcraft [options]", "Usage: easypdf [options]"),
        ("Opens the files in the PdfCraft desktop app.", "Opens the files in the easyPDF desktop app."),
        ("passing the files to a running PdfCraft", "passing the files to a running easyPDF"),
        ("(pdfcraft-cli ui --control", "(easypdf-cli ui --control"),
        ("use pdfcraft-cli (pdfcraft-cli --help)", "use easypdf-cli (easypdf-cli --help)"),
        (".with_title(\"PdfCraft\")", ".with_title(\"easyPDF\")"),
    ] {
        replace_exact_once(&desktop, old, new)?;
    }
    Ok(())
}

fn apply_web_host(root: &Path, stage: &Path, release: &str, backup_label: &str) -> Result<()> {
    // Pin the upstream entry point before applying the easyPDF-owned connected host.
    // v0.19 also supports a canvas hosted directly by a page; both transports are
    // carried in the reviewed, hash-pinned easyPDF template.
    let relative = Path::new("apps/easypdf-web/src/main.rs");
    let destination = stage.join(relative);
    let upstream = read_bounded(&destination)?;
    // The full upstream preimage is pinned so any further host change stops for review.
    let expected_upstream = if release == "v0.19.0" {
        "1dd15c1b07577d908a4332c98aa236b2f18edefdb46ae88124b61bda6682d587"
    } else {
        "9de2ee969d9cebdd40c320422be6fa7bb355aba451c8e1f92ab76b2f2657944e"
    };
    let observed_upstream = sha256(&upstream);
    if observed_upstream != expected_upstream {
        bail!("review required: upstream web host changed in {} (expected {expected_upstream}, found {observed_upstream})", destination.display());
    }
    let local = read_bounded(&root.join(relative))?;
    if sha256(&local) != "519a05cd49fc922013b0512c9ae274dcc4b4e76522e6653b5596e40ae972d272" {
        bail!("review required: easyPDF web host changed after layer extraction");
    }
    let backup = root.join("upstream/backups").join(format!("{backup_label}-fresh-web-host-before-apply")).join(relative);
    if backup.exists() {
        bail!("web-host backup already exists: {}", backup.display());
    }
    let local_text = String::from_utf8(local).context("decode easyPDF web host")?;
    let with_warning = replace_exact_text(
        &local_text,
        "                        bridge.post_ready();\n                    }\n                    Ok(Box::new(app))",
        "                        bridge.post_ready();\n                    } else if query_param(\"host\").as_deref() == Some(\"parent\") {\n                        eframe::web_sys::console::warn_1(\n                            &\"easyPDF: ?host=parent needs a framing page on the same origin; saves download instead\".into(),\n                        );\n                    }\n                    Ok(Box::new(app))",
    )?;
    let with_guard = replace_exact_text(
        &with_warning,
        "        /// `None` when the page isn't framed (there is no parent to talk to).\n        pub fn new() -> Option<Self> {",
        "        /// `None` when the page is unframed or its parent is on another origin.\n        /// Otherwise postMessage saves would disappear while the document looked saved.\n        pub fn new() -> Option<Self> {",
    )?;
    let with_guard = replace_exact_text(
        &with_guard,
        "            let origin = window.location().origin().ok()?;\n            Some(Self { window, parent, origin })",
        "            let origin = window.location().origin().ok()?;\n            // Reading a cross-origin parent's location throws; do not create a save bridge.\n            if parent.location().origin().ok()? != origin {\n                return None;\n            }\n            Some(Self { window, parent, origin })",
    )?;
    let manifest = stage.join("apps/easypdf-web/Cargo.toml");
    let manifest_backup = backup.with_file_name("Cargo.toml");
    if manifest_backup.exists() {
        bail!("web-host manifest backup already exists: {}", manifest_backup.display());
    }
    let manifest_before = read_bounded(&manifest)?;
    atomic_write(&backup, &upstream)?;
    atomic_write(&manifest_backup, &manifest_before)?;
    let web_source: &[u8] = if release == "v0.19.0" {
        let template = include_bytes!("fresh/web_host_v019.rs");
        if sha256(template) != "49de8d106e4bb8ff8bf6b82c07297c7007baf66e9fc4c94a97e114a00c905fb5" {
            bail!("review required: easyPDF v0.19 page-host template changed");
        }
        template
    } else {
        with_guard.as_bytes()
    };
    atomic_write(&destination, web_source)?;
    replace_exact_once(
        &manifest,
        "description = \"PdfCraft in the browser (wasm32; build with `trunk build --release` in this directory)\"",
        "description = \"easyPDF in the browser (wasm32; build with `trunk build --release` in this directory)\"",
    )?;
    replace_exact_once(
        &manifest,
        "egui = { workspace = true, features = [\"default_fonts\", \"monochrome_emoji_fonts\"] }",
        "egui = { workspace = true, features = [\"default_fonts\", \"monochrome_emoji_fonts\"] }\nserde_json = { workspace = true }\nbase64 = \"0.22\"",
    )?;
    let old_web_sys = if release == "v0.19.0" {
        "web-sys = { version = \"0.3\", features = [\"Window\", \"Document\", \"Element\", \"HtmlCanvasElement\", \"HtmlElement\", \"Location\", \"Url\", \"UrlSearchParams\", \"Response\", \"Event\", \"MessageEvent\", \"EventTarget\", \"CustomEvent\", \"CustomEventInit\"] }"
    } else {
        "web-sys = { version = \"0.3\", features = [\"Window\", \"Document\", \"Element\", \"HtmlCanvasElement\", \"HtmlElement\", \"Location\", \"Url\", \"UrlSearchParams\", \"Response\", \"Event\", \"MessageEvent\", \"EventTarget\"] }"
    };
    let new_web_sys = if release == "v0.19.0" {
        "web-sys = { version = \"0.3\", features = [\"Window\", \"Document\", \"Element\", \"Headers\", \"HtmlCanvasElement\", \"HtmlElement\", \"Location\", \"Request\", \"RequestInit\", \"RequestMode\", \"Url\", \"UrlSearchParams\", \"Response\", \"Event\", \"MessageEvent\", \"EventTarget\", \"CustomEvent\", \"CustomEventInit\"] }"
    } else {
        "web-sys = { version = \"0.3\", features = [\"Window\", \"Document\", \"Element\", \"Headers\", \"HtmlCanvasElement\", \"HtmlElement\", \"Location\", \"Request\", \"RequestInit\", \"RequestMode\", \"Url\", \"UrlSearchParams\", \"Response\", \"Event\", \"MessageEvent\", \"EventTarget\"] }"
    };
    replace_exact_once(&manifest, old_web_sys, new_web_sys)?;
    if release == "v0.19.0" {
        // The reviewed easyPDF bridge uses the same event contract without a new dependency.
        replace_exact_once(
            &manifest,
            "# The host messages (storytold/craft-libs): one contract and one bridge for the web builds.\ncraft-web-host = { git = \"https://github.com/storytold/craft-libs\", rev = \"2e2312ddaf21bc3d85cd3395d0955c7d239306f6\", version = \"0.1.0\" }\n",
            "",
        )?;
    }
    Ok(())
}

fn replace_range_pinned(path: &Path, start: &str, end: &str, expected_sha256: &str, replacement: &str) -> Result<()> {
    let source = String::from_utf8(read_bounded(path)?).with_context(|| format!("decode {}", path.display()))?;
    let begin = source.find(start).with_context(|| format!("review required: missing start anchor in {}", path.display()))?;
    let finish = source[begin + start.len()..]
        .find(end)
        .map(|offset| begin + start.len() + offset)
        .with_context(|| format!("review required: missing end anchor in {}", path.display()))?;
    let old = &source[begin..finish];
    if sha256(old.as_bytes()) != expected_sha256 {
        bail!("review required: upstream section changed in {}", path.display());
    }
    let mut rewritten = String::with_capacity(source.len().saturating_sub(old.len()).saturating_add(replacement.len()));
    rewritten.push_str(&source[..begin]);
    rewritten.push_str(replacement);
    rewritten.push_str(&source[finish..]);
    atomic_write(path, rewritten.as_bytes())
}

fn disable_unapproved_update_channel(stage: &Path) -> Result<()> {
    // There is no signed easyPDF update feed. A newly published upstream release must never
    // make a staged easyPDF desktop build contact or install PdfCraft releases by default.
    let main = stage.join("apps/easypdf/src/main.rs");
    replace_exact_once(
        &main,
        "    // Windows: the update helper is a copy of this program started with `--finish-update`.\n    #[cfg(windows)]\n    if let Some(result) = craft_update::self_update::finish(&easypdf_ui_egui::UPDATE_APP, &std::env::args_os().skip(1).collect::<Vec<_>>()) {\n        std::process::exit(i32::from(result.is_err()));\n    }\n",
        "    // No signed easyPDF update channel is configured; do not run upstream's helper.\n",
    )?;
    replace_exact_once(&main, "app.set_startup_update_check(true);", "app.set_startup_update_check(false);")?;
    let ui = stage.join("crates/ui-egui/src/lib.rs");
    replace_exact_once(
        &ui,
        "/// PdfCraft as craft-update knows it, with its 128 px icon and accent colour: the update\n/// prompt shows both, and an AppImage's menu entry gets the icon.\n#[cfg(not(target_arch = \"wasm32\"))]\npub const UPDATE_APP: craft_update::app::App = craft_update::app::App {\n    icon_png: Some(include_bytes!(\"../../../assets/app-icon/hicolor/128x128/apps/it.twinba.easypdf.png\")),\n    accent: Some([0xb7, 0x02, 0x28]),\n    ..craft_update::suite::PDFCRAFT\n};",
        "/// easyPDF identity retained for compilation; update checks stay disabled until an\n/// easyPDF-owned, signed release channel is configured.\n#[cfg(not(target_arch = \"wasm32\"))]\npub const UPDATE_APP: craft_update::app::App = craft_update::app::App {\n    id: \"easypdf\",\n    name: \"easyPDF\",\n    tagline: \"PDF tools\",\n    repo: \"LFA-Asteriti/easyPDF\",\n    former_ids: &[],\n    former_names: &[],\n    icon_png: Some(include_bytes!(\"../../../assets/app-icon/hicolor/128x128/apps/it.twinba.easypdf.png\")),\n    accent: Some([0x4e, 0xb3, 0xeb]),\n    developer: \"Massimo Bertolucci\",\n};",
    )?;
    let menus = stage.join("crates/ui-egui/src/menus.rs");
    replace_exact_once(&menus, "\"help.check_updates\", SEP, ", "")?;
    let commands = stage.join("crates/ui-egui/src/commands.rs");
    replace_exact_once(&commands, "            \"help.check_updates\" => self.check_for_updates(),\n", "")?;
    let updates = stage.join("crates/ui-egui/src/platform/updates.rs");
    replace_exact_once(&updates, "https://github.com/storytold/pdfcraft/releases", "https://github.com/LFA-Asteriti/easyPDF/releases")?;
    replace_exact_once(
        &updates,
        "    pub fn check_for_updates(&mut self) {\n        #[cfg(not(target_arch = \"wasm32\"))]\n        {\n            self.updates.open = true;\n            self.updates.check_requested = true;\n        }\n        #[cfg(target_arch = \"wasm32\")]\n        {\n            self.open_url(RELEASES_PAGE);\n        }\n    }",
        "    pub fn check_for_updates(&mut self) {\n        // Deliberately unavailable until easyPDF has its own signed update channel.\n    }",
    )?;
    Ok(())
}

fn replace_exact_once(path: &Path, old: &str, new: &str) -> Result<()> {
    let bytes = read_bounded(path)?;
    let source = String::from_utf8(bytes).with_context(|| format!("decode {}", path.display()))?;
    let rewritten = replace_exact_text(&source, old, new).with_context(|| format!("review exact fresh-layer patch in {}", path.display()))?;
    atomic_write(path, rewritten.as_bytes())
}

fn replace_exact_text(source: &str, old: &str, new: &str) -> Result<String> {
    if source.matches(old).count() != 1 {
        bail!("review required: expected exactly one fresh-layer anchor");
    }
    Ok(source.replacen(old, new, 1))
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() > 32 * 1024 * 1024 {
        bail!("fresh layer input is not a regular file below 32 MiB: {}", path.display());
    }
    std::fs::read(path).with_context(|| format!("read {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::replace_exact_text;

    #[test]
    fn update_guard_replaces_exactly_one_match() {
        assert_eq!(replace_exact_text("before OLD after", "OLD", "NEW").unwrap(), "before NEW after");
        assert!(replace_exact_text("before after", "OLD", "NEW").is_err());
        assert!(replace_exact_text("OLD and OLD", "OLD", "NEW").is_err());
    }
}
