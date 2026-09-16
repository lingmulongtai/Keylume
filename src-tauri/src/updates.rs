use semver::Version;
use serde::{Deserialize, Serialize};

pub const REPOSITORY: &str = "https://github.com/lingmulongtai/Keylume";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    pub tag: String,
    pub url: String,
    pub prerelease: bool,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    state: String,
    size: u64,
    browser_download_url: String,
}
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

pub fn select_release(
    bytes: &[u8],
    current: &str,
    include_previews: bool,
) -> Result<Option<Release>, String> {
    let current = Version::parse(current).map_err(|_| "現在のバージョンが不正です")?;
    let releases: Vec<GithubRelease> =
        serde_json::from_slice(bytes).map_err(|_| "GitHub の応答を読み取れませんでした")?;
    Ok(releases
        .into_iter()
        .filter_map(|r| {
            let version = Version::parse(r.tag_name.strip_prefix('v')?).ok()?;
            let preview = r.prerelease || !version.pre.is_empty();
            if r.draft || (!include_previews && preview) || version <= current {
                return None;
            }
            let asset = format!("Keylume_{version}_x64-setup.exe");
            let download = format!("{REPOSITORY}/releases/download/{}/{asset}", r.tag_name);
            if !r.assets.iter().any(|a| {
                a.name == asset
                    && a.state == "uploaded"
                    && a.size > 0
                    && a.browser_download_url == download
            }) {
                return None;
            }
            let release = Release {
                version: version.to_string(),
                url: format!("{REPOSITORY}/releases/tag/{}", r.tag_name),
                tag: r.tag_name,
                prerelease: preview,
            };
            Some((version, release))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, r)| r))
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateHistory {
    pub checked_at: Option<String>,
    pub dismissed_version: Option<String>,
    pub notified_version: Option<String>,
}
impl UpdateHistory {
    pub fn should_notify(&self, release: &Release) -> bool {
        self.dismissed_version.as_deref() != Some(&release.version)
            && self.notified_version.as_deref() != Some(&release.version)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub status: String,
    pub current_version: String,
    pub release: Option<Release>,
    pub error: Option<String>,
    #[serde(flatten)]
    pub history: UpdateHistory,
}
impl UpdateState {
    pub fn new(history: UpdateHistory) -> Self {
        Self {
            status: "idle".into(),
            current_version: env!("CARGO_PKG_VERSION").into(),
            release: None,
            error: None,
            history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn release(version: &str, preview: bool) -> Value {
        json!({"tag_name":format!("v{version}"),"draft":false,"prerelease":preview,"assets":[{"name":format!("Keylume_{version}_x64-setup.exe"),"state":"uploaded","size":100,"browser_download_url":format!("{REPOSITORY}/releases/download/v{version}/Keylume_{version}_x64-setup.exe")} ]})
    }
    fn select(values: Vec<Value>, previews: bool) -> Option<Release> {
        select_release(&serde_json::to_vec(&values).unwrap(), "0.1.9", previews).unwrap()
    }
    #[test]
    fn semantic_versions_and_preview_channel_are_respected() {
        assert_eq!(
            select(
                vec![
                    release("0.1.10", false),
                    release("0.1.9", false),
                    release("0.2.0", true)
                ],
                false
            )
            .unwrap()
            .version,
            "0.1.10"
        );
        assert_eq!(
            select(vec![release("0.1.10", false), release("0.2.0", true)], true)
                .unwrap()
                .version,
            "0.2.0"
        );
        assert!(select(vec![release("0.1.9", false), release("0.1.8", false)], true).is_none());
        assert!(select(vec![release("0.2.0-beta.1", false)], false).is_none());
    }
    #[test]
    fn drafts_missing_assets_and_foreign_urls_cannot_prompt() {
        for field in ["draft", "asset", "url", "tag", "size"] {
            let mut r = release("0.2.0", true);
            match field {
                "draft" => r["draft"] = true.into(),
                "asset" => r["assets"] = json!([]),
                "url" => {
                    r["assets"][0]["browser_download_url"] = "https://example.com/setup.exe".into()
                }
                "tag" => r["tag_name"] = "../../unexpected".into(),
                _ => r["assets"][0]["size"] = 0.into(),
            }
            assert!(select(vec![r], true).is_none(), "{field}");
        }
        assert!(select_release(b"not json", "0.1.9", true).is_err());
    }
    #[test]
    fn dismissal_and_notification_are_per_version() {
        let r = select(vec![release("0.2.0", true)], true).unwrap();
        let mut history = UpdateHistory::default();
        assert!(history.should_notify(&r));
        history.dismissed_version = Some(r.version.clone());
        assert!(!history.should_notify(&r));
        let newer = select(vec![release("0.3.0", true)], true).unwrap();
        assert!(history.should_notify(&newer));
        history.notified_version = Some(newer.version.clone());
        assert!(!history.should_notify(&newer));
    }
    #[test]
    fn existing_settings_enable_update_checks_by_default() {
        let settings: crate::model::Settings =
            serde_json::from_str(r#"{"schema":1,"masterBrightness":0.4}"#).unwrap();
        assert!(settings.check_for_updates && settings.include_prereleases);
        assert_eq!(settings.master_brightness, 0.4);
    }
}
