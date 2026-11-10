//! Release-based update checking and in-app installation.
//!
//! The app ships three ways from the GitHub Release of the matching tag:
//! a `.deb` package, a portable `.AppImage` and a Flatpak bundle. The
//! update flow queries the latest published release, compares its tag with
//! the running version and installs the matching artifact:
//!
//! - `.deb` installs are handed to `pkexec apt-get install`, so the usual
//!   package manager keeps ownership of `/usr/bin/takto`.
//! - `.AppImage` installs replace the running image in place, after which
//!   the app offers to restart itself.
//! - Flatpak sandboxes and development builds cannot self-install; they
//!   open the release page instead of downloading anything.
//!
//! Everything here is UI-free so it compiles (and is tested) without the
//! `ui` feature.

use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;

pub const RELEASES_API_URL: &str = "https://api.github.com/repos/ez404-36/takto/releases/latest";
pub const RELEASES_PAGE_URL: &str = "https://github.com/ez404-36/takto/releases/latest";

/// Marker file the Flatpak runtime always mounts inside the sandbox.
const FLATPAK_INFO_PATH: &str = "/.flatpak-info";

const USER_AGENT: &str = concat!("takto/", env!("CARGO_PKG_VERSION"));
/// Whole-request budget for the release lookup: one small JSON document.
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
/// Dial + per-read budgets for artifact downloads, which may be tens of MB.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Guard against a broken server streaming an unbounded body.
const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

/// The latest published (non-draft, non-prerelease) GitHub Release.
#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub page_url: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Clone, Debug)]
pub struct ReleaseAsset {
    pub kind: AssetKind,
    pub url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Deb,
    AppImage,
}

/// How the running binary was deployed; decides whether an update can be
/// installed from inside the app or only pointed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallContext {
    /// The portable image itself is replaced, then the app restarts.
    AppImage,
    /// The `.deb` is installed through `pkexec apt-get install`.
    Debian,
    /// Sandboxed bundles must be updated through their own tooling.
    Flatpak,
    /// A `cargo run` build: updates make no sense.
    Development,
    /// Could not be determined; the release page is the safe fallback.
    Unknown,
}

impl Release {
    pub fn asset_url(&self, kind: AssetKind) -> Option<&str> {
        self.assets
            .iter()
            .find(|asset| asset.kind == kind)
            .map(|asset| asset.url.as_str())
    }
}

/// Detects how this instance was installed from its own runtime markers.
pub fn detect_install_context() -> InstallContext {
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let executable = std::env::current_exe().ok();
    let flatpak = Path::new(FLATPAK_INFO_PATH).exists();
    detect_context(appimage.as_deref(), executable.as_deref(), flatpak)
}

/// Pure classification behind [`detect_install_context`], testable without
/// touching the real environment.
pub fn detect_context(
    appimage: Option<&Path>,
    executable: Option<&Path>,
    flatpak: bool,
) -> InstallContext {
    if flatpak {
        return InstallContext::Flatpak;
    }
    if appimage.is_some_and(|image| image.is_file()) {
        return InstallContext::AppImage;
    }
    match executable {
        // `/usr/bin/takto` is what the .deb package installs.
        Some(executable) if executable.starts_with("/usr") => InstallContext::Debian,
        Some(_) => InstallContext::Development,
        None => InstallContext::Unknown,
    }
}

/// Fetches the latest published release from the GitHub REST API.
pub fn fetch_latest_release() -> anyhow::Result<Release> {
    let agent = ureq::AgentBuilder::new()
        .timeout(CHECK_TIMEOUT)
        .user_agent(USER_AGENT)
        .build();
    let response = agent
        .get(RELEASES_API_URL)
        .set("Accept", "application/vnd.github+json")
        .call()
        .context("could not reach the GitHub release API")?;

    let mut body = String::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES)
        .read_to_string(&mut body)
        .context("could not read the release response")?;
    parse_release(&body)
}

/// Parses the GitHub release JSON payload.
pub fn parse_release(body: &str) -> anyhow::Result<Release> {
    let value: serde_json::Value =
        serde_json::from_str(body).context("release response is not valid JSON")?;
    let tag = value
        .get("tag_name")
        .and_then(|value| value.as_str())
        .context("release response has no tag_name")?;
    let version = tag.trim().trim_start_matches('v').to_string();
    anyhow::ensure!(!version.is_empty(), "release tag {tag} has no version");

    let page_url = value
        .get("html_url")
        .and_then(|value| value.as_str())
        .unwrap_or(RELEASES_PAGE_URL)
        .to_string();

    let mut assets = Vec::new();
    if let Some(list) = value.get("assets").and_then(|value| value.as_array()) {
        for asset in list {
            let Some(name) = asset.get("name").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(url) = asset
                .get("browser_download_url")
                .and_then(|value| value.as_str())
            else {
                continue;
            };
            if let Some(kind) = asset_kind(name) {
                assets.push(ReleaseAsset {
                    kind,
                    url: url.to_string(),
                });
            }
        }
    }

    Ok(Release {
        version,
        page_url,
        assets,
    })
}

/// Recognizes the release artifacts this app can install by file extension.
pub fn asset_kind(name: &str) -> Option<AssetKind> {
    if name.ends_with(".AppImage") {
        Some(AssetKind::AppImage)
    } else if name.ends_with(".deb") {
        Some(AssetKind::Deb)
    } else {
        None
    }
}

/// Whether `latest` is a strictly newer release than `current`.
///
/// Dotted numeric comparison with `v` prefixes tolerated. Trailing missing
/// components count as zero, so `0.1 < 0.1.4`; versions produced by this
/// repository always have three components anyway.
pub fn is_newer(latest: &str, current: &str) -> bool {
    version_numbers(latest) > version_numbers(current)
}

fn version_numbers(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().unwrap_or(0)
        })
        .collect()
}

/// Where a downloaded artifact lands while the install runs.
pub fn download_dest(kind: AssetKind, version: &str) -> PathBuf {
    let name = match kind {
        AssetKind::Deb => format!("takto-{version}.deb"),
        AssetKind::AppImage => format!("takto-{version}.AppImage"),
    };
    std::env::temp_dir().join(name)
}

/// Path of the running AppImage, when this instance is one.
pub fn appimage_path() -> anyhow::Result<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .context("APPIMAGE environment variable is not set or does not point to a file")
}

/// Streams `url` into `dest`, reporting progress as
/// `(total bytes, downloaded bytes)`; total is `None` without a
/// Content-Length header.
pub fn download_to(
    url: &str,
    dest: &Path,
    progress: &mut dyn FnMut(Option<u64>, u64),
) -> anyhow::Result<()> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout_read(READ_TIMEOUT)
        .user_agent(USER_AGENT)
        .build();
    let response = agent.get(url).call().context("download request failed")?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());

    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(dest)
        .with_context(|| format!("could not create {}", dest.display()))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let read = reader.read(&mut buffer).context("download failed")?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .with_context(|| format!("could not write {}", dest.display()))?;
        downloaded += read as u64;
        progress(total, downloaded);
    }
    file.flush()
        .context("could not finish writing the download")?;
    Ok(())
}

/// Atomically replaces the running AppImage with `downloaded`.
///
/// The new image is staged next to the running one (same filesystem, so the
/// final rename cannot cross mount points) and made executable first; the
/// kernel keeps serving the running process from the old inode until exit.
pub fn install_appimage(appimage: &Path, downloaded: &Path) -> anyhow::Result<()> {
    let parent = appimage
        .parent()
        .context("the running AppImage has no parent directory")?;
    let file_name = appimage
        .file_name()
        .context("the running AppImage has no file name")?
        .to_string_lossy()
        .into_owned();
    let staged = parent.join(format!(".{file_name}.update"));
    std::fs::copy(downloaded, &staged)
        .with_context(|| format!("could not stage {}", staged.display()))?;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("could not mark {} executable", staged.display()))?;
    std::fs::rename(&staged, appimage)
        .with_context(|| format!("could not replace {}", appimage.display()))?;
    Ok(())
}

/// Installs a downloaded `.deb` through the package manager.
///
/// `pkexec` shows the system authentication dialog; `apt-get` resolves the
/// package's `$auto` runtime dependencies, which `dpkg -i` would not.
pub fn install_debian(deb_path: &Path) -> anyhow::Result<()> {
    let status = std::process::Command::new("pkexec")
        .args(["/usr/bin/apt-get", "install", "-y"])
        .arg(deb_path)
        .status()
        .context("could not run pkexec to install the package")?;
    anyhow::ensure!(status.success(), "package installation failed: {status}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live-API sanity check. Kept out of the default run because CI must
    /// stay hermetic; run explicitly with `cargo test -- --ignored`.
    #[test]
    #[ignore = "hits the network"]
    fn fetch_latest_release_parses_the_live_api() {
        let release = fetch_latest_release().expect("live GitHub release lookup should work");
        assert!(
            release
                .version
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_digit()),
            "version should start with a digit, got {}",
            release.version
        );
        assert!(release.asset_url(AssetKind::Deb).is_some());
        assert!(release.asset_url(AssetKind::AppImage).is_some());
    }

    #[test]
    fn version_comparison_is_numeric_dotted() {
        assert!(is_newer("0.1.5", "0.1.4"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.4", "0.1.4"));
        assert!(!is_newer("0.1.3", "0.1.4"));
        assert!(!is_newer("v0.1.4", "0.1.4"));
        // Trailing components default to zero: 0.1 < 0.1.4.
        assert!(is_newer("0.1.4", "0.1"));
    }

    #[test]
    fn release_payload_is_parsed_with_installable_assets() {
        let body = r#"{
            "tag_name": "v0.2.0",
            "html_url": "https://github.com/ez404-36/takto/releases/tag/v0.2.0",
            "assets": [
                {
                    "name": "takto_0.2.0-1_amd64.deb",
                    "browser_download_url": "https://example.com/takto_0.2.0-1_amd64.deb"
                },
                {
                    "name": "takto_0.2.0_x86_64.AppImage",
                    "browser_download_url": "https://example.com/takto_0.2.0_x86_64.AppImage"
                },
                {
                    "name": "source.tar.gz",
                    "browser_download_url": "https://example.com/source.tar.gz"
                }
            ]
        }"#;

        let release = parse_release(body).unwrap();
        assert_eq!(release.version, "0.2.0");
        assert_eq!(
            release.page_url,
            "https://github.com/ez404-36/takto/releases/tag/v0.2.0"
        );
        assert_eq!(release.assets.len(), 2);
        assert_eq!(
            release.asset_url(AssetKind::Deb),
            Some("https://example.com/takto_0.2.0-1_amd64.deb")
        );
        assert_eq!(
            release.asset_url(AssetKind::AppImage),
            Some("https://example.com/takto_0.2.0_x86_64.AppImage")
        );
    }

    #[test]
    fn malformed_release_payloads_are_rejected() {
        assert!(parse_release("not json").is_err());
        assert!(parse_release(r#"{"assets": []}"#).is_err());
    }

    #[test]
    fn install_context_follows_runtime_markers() {
        let image = std::env::temp_dir().join("takto-context-test.AppImage");
        std::fs::write(&image, b"").unwrap();

        assert_eq!(
            detect_context(Some(&image), Some(Path::new("/usr/bin/takto")), false),
            InstallContext::AppImage
        );
        assert_eq!(
            detect_context(None, Some(Path::new("/usr/bin/takto")), false),
            InstallContext::Debian
        );
        assert_eq!(
            detect_context(Some(&image), Some(Path::new("/usr/bin/takto")), true),
            InstallContext::Flatpak
        );
        assert_eq!(
            detect_context(
                None,
                Some(Path::new("/home/x/takto/target/debug/takto")),
                false
            ),
            InstallContext::Development
        );
        assert_eq!(detect_context(None, None, false), InstallContext::Unknown);

        std::fs::remove_file(&image).unwrap();
    }

    #[test]
    fn asset_kind_matches_release_file_names() {
        assert_eq!(asset_kind("takto_0.1.4-1_amd64.deb"), Some(AssetKind::Deb));
        assert_eq!(
            asset_kind("takto_0.1.4_x86_64.AppImage"),
            Some(AssetKind::AppImage)
        );
        assert_eq!(asset_kind("takto_0.1.4_x86_64.flatpak"), None);
    }
}
