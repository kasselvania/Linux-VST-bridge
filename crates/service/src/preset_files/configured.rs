//! Read one declared path preference. Unrelated values are skipped and never
//! enter a receipt, source binding, diagnostic or backup.
use super::{fs, require, Result};
use serde::{
    de::{DeserializeSeed, IgnoredAny, MapAccess, Visitor},
    Deserialize, Serialize,
};
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    file: PathBuf,
    key: String,
    value: String,
    drive_c: PathBuf,
    suffix: String,
}
impl Source {
    pub(super) fn observe(
        setup: &Path,
        file: &str,
        key: &str,
        drive_c: &str,
        suffix: &str,
    ) -> Result<Self> {
        let file = setup.join(file);
        let value = selected(&file, key)?;
        let source = Self {
            file,
            key: key.into(),
            value,
            drive_c: setup.join(drive_c),
            suffix: suffix.into(),
        };
        source.root()?;
        Ok(source)
    }
    pub(super) fn root(&self) -> Result<PathBuf> {
        // No drive discovery, Wine mapping inference, environment expansion,
        // UNC paths or traversal. Other configured roots remain unavailable.
        let normalized = self.value.replace('\\', "/");
        let relative = normalized
            .strip_prefix("C:/")
            .or_else(|| normalized.strip_prefix("c:/"))
            .ok_or("preset_configured_drive_not_qualified")?
            .trim_end_matches('/');
        fs::relative(relative)?;
        fs::relative(&self.suffix)?;
        Ok(self.drive_c.join(relative).join(&self.suffix))
    }
    pub(super) fn revalidate(&self) -> Result<()> {
        require(
            selected(&self.file, &self.key)? == self.value,
            "preset_configured_root_changed_refresh",
        )
    }
}

fn selected(path: &Path, key: &str) -> Result<String> {
    let parent = fs::directory(path.parent().ok_or("preset_preference_parent")?)?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("preset_preference_name")?;
    let file = fs::at(&parent, name, libc::O_RDONLY, 0)?;
    require(
        file.metadata()?.len() <= 1024 * 1024,
        "preset_preference_size_bound",
    )?;
    let identity = fs::stamp(&file.metadata()?);
    let bytes = fs::bytes(&file)?;
    let mut decoder = serde_json::Deserializer::from_slice(&bytes);
    let value = Select(key)
        .deserialize(&mut decoder)
        .map_err(|_| "preset_preference_invalid")?;
    decoder.end().map_err(|_| "preset_preference_invalid")?;
    require(value.len() <= 4096, "preset_preference_path_bound")?;
    let current = fs::at(&parent, name, libc::O_RDONLY, 0)?;
    require(
        identity == fs::stamp(&file.metadata()?) && identity == fs::stamp(&current.metadata()?),
        "preset_preference_changed_refresh",
    )?;
    Ok(value)
}
struct Select<'a>(&'a str);
impl<'de> DeserializeSeed<'de> for Select<'_> {
    type Value = String;
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        decoder: D,
    ) -> std::result::Result<String, D::Error> {
        decoder.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for Select<'_> {
    type Value = String;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a preference object")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> std::result::Result<String, M::Error> {
        let mut selected = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == self.0 {
                if selected.is_some() {
                    return Err(serde::de::Error::custom("duplicate path preference"));
                }
                selected = Some(map.next_value::<String>()?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        selected.ok_or_else(|| serde::de::Error::custom("missing path preference"))
    }
}
