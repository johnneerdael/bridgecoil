//! Apps published as several APKs reach people as one archive: `.apks`, `.xapk`, `.apkm`, `.apk+`
//! or a plain `.zip`. They're all ZIPs of APKs, so the name of the file decides nothing here. Each
//! APK's own manifest says what it is, which is also what Android goes by when it installs them.
use crate::{
    apk,
    apk_set::{self, ApkSetInfo, SignedApkInfo},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

/// Game data makes these far larger than any APK.
pub const MAX_BUNDLE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;
const MAX_APKS: usize = 512;
/// What may land on the phone while parts are sorted, before any are dropped.
const MAX_UNPACKED_APK_BYTES: u64 = 3 * 1024 * 1024 * 1024;
const PASSWORD: &str = "This file is password protected, so Bridgecoil can't open it. Download a version without a password.";
const UNREADABLE: &str = "This file isn't an APK or an app bundle Bridgecoil can read.";

/// What decides which parts a TV needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceProfile {
    pub sdk: u32,
    /// Best first, as the TV lists them.
    pub abis: Vec<String>,
    pub density: u32,
    pub languages: Vec<String>,
}

/// Reads the output of [`PROFILE_COMMAND`].
pub fn parse_profile(sdk: u32, abis: Vec<String>, raw: &str) -> DeviceProfile {
    let number = |line: &str| line.rsplit(' ').next()?.trim().parse::<u32>().ok();
    let lines = || raw.lines().map(str::trim);
    let density = lines()
        .find(|l| l.starts_with("Override density"))
        .or_else(|| lines().find(|l| l.starts_with("Physical density")))
        .or_else(|| lines().find(|l| !l.is_empty() && l.bytes().all(|c| c.is_ascii_digit())))
        .and_then(number)
        .unwrap_or(0);
    let mut languages = Vec::new();
    for tag in lines()
        .filter_map(|l| l.strip_prefix("locales "))
        .flat_map(|l| l.split(','))
    {
        let language = tag.trim().split(['-', '_']).next().unwrap_or("").to_lowercase();
        if (2..=3).contains(&language.len())
            && language.bytes().all(|c| c.is_ascii_lowercase())
            && !languages.contains(&language)
        {
            languages.push(language);
        }
    }
    DeviceProfile { sdk, abis, density, languages }
}
pub const PROFILE_COMMAND: &str = "wm density 2>/dev/null; getprop ro.sf.lcd_density; \
    echo locales $(settings get system system_locales 2>/dev/null),$(getprop persist.sys.locale),$(getprop ro.product.locale); true";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Config {
    Abi(&'static str),
    Density(u32),
    Language(String),
}

fn abi(value: &str) -> Option<&'static str> {
    Some(match value {
        "armeabi" => "armeabi",
        "armeabi_v7a" => "armeabi-v7a",
        "arm64_v8a" => "arm64-v8a",
        "x86" => "x86",
        "x86_64" => "x86_64",
        "mips" => "mips",
        "mips64" => "mips64",
        "riscv64" => "riscv64",
        _ => return None,
    })
}

fn density(value: &str) -> Option<u32> {
    Some(match value {
        "ldpi" => 120,
        "mdpi" => 160,
        "tvdpi" => 213,
        "hdpi" => 240,
        "xhdpi" => 320,
        "xxhdpi" => 480,
        "xxxhdpi" => 640,
        _ => return None,
    })
}

fn language(value: &str) -> Option<String> {
    let first = value.split('_').next()?;
    ((2..=3).contains(&first.len()) && first.bytes().all(|c| c.is_ascii_lowercase()))
        .then(|| first.to_owned())
}

/// `config.xhdpi` belongs to the app, `feature.config.xhdpi` to that feature.
fn config_of(split: &str) -> Option<(&str, Config)> {
    let (scope, value) = match split.strip_prefix("config.") {
        Some(value) => ("", value),
        None => split.rsplit_once(".config.")?,
    };
    let config = abi(value)
        .map(Config::Abi)
        .or_else(|| density(value).map(Config::Density))
        .or_else(|| language(value).map(Config::Language))?;
    Some((scope, config))
}

/// Picks what the TV needs, by the rules Android itself uses to choose resources. Anything that
/// isn't recognised stays, because a part too many costs space and a part too few breaks the app.
pub fn choose(splits: &[Option<String>], profile: &DeviceProfile) -> Result<Vec<usize>, String> {
    let mut scopes: BTreeMap<&str, Vec<(usize, Config)>> = BTreeMap::new();
    let mut keep: Vec<usize> = Vec::new();
    for (index, split) in splits.iter().enumerate() {
        match split.as_deref().and_then(config_of) {
            Some((scope, config)) => scopes.entry(scope).or_default().push((index, config)),
            None => keep.push(index),
        }
    }
    for parts in scopes.values() {
        let abis: Vec<_> = parts
            .iter()
            .filter_map(|(i, c)| matches!(c, Config::Abi(_)).then_some((*i, c)))
            .collect();
        match profile.abis.iter().find(|wanted| {
            abis.iter().any(|(_, c)| matches!(c, Config::Abi(a) if a == wanted))
        }) {
            Some(wanted) => keep.extend(
                abis.iter()
                    .filter(|(_, c)| matches!(c, Config::Abi(a) if a == wanted))
                    .map(|(i, _)| *i),
            ),
            None if abis.is_empty() => {}
            None if profile.abis.is_empty() => keep.extend(abis.iter().map(|(i, _)| *i)),
            None => {
                return Err(format!(
                    "This file has no version for this TV's processor ({}). Look for a build that includes it.",
                    profile.abis.join(", ")
                ));
            }
        }
        let dpis: Vec<_> = parts
            .iter()
            .filter_map(|(i, c)| match c {
                Config::Density(d) => Some((*i, *d)),
                _ => None,
            })
            .collect();
        let best = dpis
            .iter()
            .map(|(_, d)| *d)
            .filter(|d| *d >= profile.density)
            .min()
            .or_else(|| dpis.iter().map(|(_, d)| *d).max());
        keep.extend(
            dpis.iter()
                .filter(|(_, d)| profile.density == 0 || Some(*d) == best)
                .map(|(i, _)| *i),
        );
        let languages: Vec<_> = parts
            .iter()
            .filter_map(|(i, c)| match c {
                Config::Language(l) => Some((*i, l.as_str())),
                _ => None,
            })
            .collect();
        let wanted: Vec<_> = languages
            .iter()
            .filter(|(_, l)| *l == "en" || profile.languages.iter().any(|w| w == l))
            .map(|(i, _)| *i)
            .collect();
        if wanted.is_empty() {
            keep.extend(languages.iter().map(|(i, _)| *i));
        } else {
            keep.extend(wanted);
        }
    }
    keep.sort_unstable();
    Ok(keep)
}

fn is_apk(name: &str) -> bool {
    name.to_lowercase().ends_with(".apk") && !name.starts_with("__MACOSX/")
}

/// bundletool's `.apks` also holds whole-app builds for older Android, next to the real splits.
fn apk_entries(names: &[String]) -> Vec<usize> {
    let apks: Vec<usize> = (0..names.len()).filter(|i| is_apk(&names[*i])).collect();
    if apks.iter().any(|i| names[*i].starts_with("splits/")) {
        let whole = |n: &str| {
            n.starts_with("standalones/") || n.starts_with("instant/") || n == "universal.apk"
        };
        return apks.into_iter().filter(|i| !whole(&names[*i])).collect();
    }
    match apks.iter().find(|i| names[**i] == "universal.apk") {
        Some(universal) => vec![*universal],
        None => apks,
    }
}

/// Android looks game data up by file name, so that's all that's kept of an archive's path. The
/// folder comes from the app's verified package name, so an entry can't choose where it lands.
fn obb_name(entry: &str) -> Option<&str> {
    let name = entry.rsplit('/').next()?;
    (name.to_lowercase().ends_with(".obb")
        && name.len() <= 200
        && !name.starts_with('.')
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)))
    .then_some(name)
}

fn open(file: &Path) -> Result<ZipArchive<File>, String> {
    let archive = ZipArchive::new(File::open(file).map_err(|e| e.to_string())?)
        .map_err(|_| UNREADABLE.to_owned())?;
    if archive.len() > MAX_ENTRIES {
        return Err("This file holds too many items to be an app.".into());
    }
    Ok(archive)
}

/// Whether a chosen file needs unpacking. A lone APK is itself a ZIP, and has a manifest on top.
pub fn is_bundle(file: &Path) -> Result<bool, String> {
    let archive = open(file)?;
    let has = |name: &str| archive.file_names().any(|n| n == name);
    if has("AndroidManifest.xml") {
        return Ok(false);
    }
    if has("BundleConfig.pb") || has("base/manifest/AndroidManifest.xml") {
        return Err(
            "This is an Android App Bundle (.aab), which is what developers send to app stores. Android can't install one directly. Look for an APK, APKS, XAPK or APKM.".into(),
        );
    }
    Ok(true)
}

fn extract<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
    to: &Path,
    limit: u64,
) -> Result<u64, String> {
    let too_big = "An item inside this file is too large.";
    let mut entry = archive.by_index(index).map_err(|_| UNREADABLE.to_owned())?;
    if entry.size() > limit {
        return Err(too_big.into());
    }
    let mut out = File::create(to).map_err(|e| e.to_string())?;
    // The size above is only what the archive claims. This is what stops a ZIP bomb.
    let written = io::copy(&mut entry.by_ref().take(limit + 1), &mut out)
        .map_err(|_| "This file is damaged, so its contents can't be read.".to_owned())?;
    if written > limit {
        drop(out);
        let _ = std::fs::remove_file(to);
        return Err(too_big.into());
    }
    Ok(written)
}

fn hash(file: &Path) -> Result<String, String> {
    let mut source = File::open(file).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    io::copy(&mut source, &mut digest).map_err(|e| e.to_string())?;
    Ok(hex::encode(digest.finalize()))
}

#[derive(Debug, Clone)]
pub struct ObbFile {
    pub name: String,
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug)]
pub struct Unpacked {
    /// The base comes first.
    pub set: ApkSetInfo,
    /// In the same order as `set.files`.
    pub paths: Vec<PathBuf>,
    pub obb: Vec<ObbFile>,
    /// How many APKs the file held, of which `set.files` are the ones this TV needs.
    pub available: usize,
    /// Of the whole file, so it can be compared with what the person downloaded.
    pub sha256: String,
}

/// Unpacks into `folder`, which belongs to the caller. Files are numbered there, so nothing an
/// archive calls itself ever becomes a path on this device.
pub fn unpack(file: &Path, folder: &Path, profile: &DeviceProfile) -> Result<Unpacked, String> {
    let mut archive = open(file)?;
    let names: Vec<String> = (0..archive.len())
        .map(|i| {
            archive
                .name_for_index(i)
                .map(|n| n.replace('\\', "/"))
                .unwrap_or_default()
        })
        .collect();
    let apks = apk_entries(&names);
    if apks.is_empty() {
        return Err("There's no APK inside this file.".into());
    }
    if apks.len() > MAX_APKS {
        return Err("This file holds too many APKs to be one app.".into());
    }
    let obbs: Vec<_> = (0..names.len())
        .filter(|i| names[*i].to_lowercase().ends_with(".obb"))
        .collect();
    for index in apks.iter().chain(&obbs) {
        let entry = archive.by_index_raw(*index).map_err(|_| UNREADABLE.to_owned())?;
        if entry.encrypted() {
            return Err(PASSWORD.into());
        }
    }
    let mut parts = Vec::with_capacity(apks.len());
    let mut unpacked = 0u64;
    for (number, index) in apks.iter().enumerate() {
        let path = folder.join(format!("part-{number}.apk"));
        unpacked = unpacked.saturating_add(extract(&mut archive, *index, &path, apk::MAX_APK_BYTES)?);
        if unpacked > MAX_UNPACKED_APK_BYTES {
            return Err("The APKs inside this file are too large together.".into());
        }
        let info = apk::inspect_package(&path)
            .map_err(|e| format!("An APK inside this file can't be read: {e}"))?;
        parts.push((path, info));
    }
    let bases = parts.iter().filter(|(_, i)| i.split.is_none()).count();
    if bases == 0 {
        return Err("This file has no base APK, so it's only part of an app.".into());
    }
    if bases > 1 {
        return Err(format!(
            "This file holds {bases} separate APKs that each stand alone, so Bridgecoil can't tell which one to install."
        ));
    }
    let splits: Vec<_> = parts.iter().map(|(_, i)| i.split.clone()).collect();
    let keep = choose(&splits, profile)?;
    let mut files: Vec<SignedApkInfo> = Vec::with_capacity(keep.len());
    let mut places = HashMap::new();
    for (index, (path, info)) in parts.iter().enumerate() {
        if keep.contains(&index) {
            files.push(apk_set::inspect_part(path)?);
            places.insert(info.sha256.clone(), path.clone());
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
    let set = apk_set::assemble_set(files, profile.sdk, &profile.abis)?;
    let paths = set
        .files
        .iter()
        .map(|f| places[&f.info.sha256].clone())
        .collect();
    let mut obb: Vec<ObbFile> = Vec::with_capacity(obbs.len());
    let mut room = MAX_BUNDLE_BYTES;
    for (number, index) in obbs.iter().enumerate() {
        let name = obb_name(&names[*index])
            .ok_or("A game data file inside this has a name Android can't use.")?;
        if obb.iter().any(|o| o.name == name) {
            return Err("This file holds two game data files with the same name.".into());
        }
        let path = folder.join(format!("game-{number}.obb"));
        let bytes = extract(&mut archive, *index, &path, room)?;
        room -= bytes;
        obb.push(ObbFile { name: name.to_owned(), path, bytes });
    }
    Ok(Unpacked {
        set,
        paths,
        obb,
        available: apks.len(),
        sha256: hash(file)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tv() -> DeviceProfile {
        DeviceProfile {
            sdk: 34,
            abis: vec!["arm64-v8a".into(), "armeabi-v7a".into(), "armeabi".into()],
            density: 320,
            languages: vec!["nl".into()],
        }
    }
    fn picked(names: &[&str], profile: &DeviceProfile) -> Vec<String> {
        let splits: Vec<_> = names
            .iter()
            .map(|n| (!n.is_empty()).then(|| n.to_string()))
            .collect();
        choose(&splits, profile)
            .unwrap()
            .into_iter()
            .map(|i| names[i].to_owned())
            .collect()
    }

    #[test]
    fn keeps_the_base_and_only_what_this_tv_can_use() {
        let parts = [
            "",
            "config.arm64_v8a",
            "config.armeabi_v7a",
            "config.x86_64",
            "config.mdpi",
            "config.xhdpi",
            "config.xxhdpi",
            "config.en",
            "config.nl",
            "config.de",
        ];
        assert_eq!(
            picked(&parts, &tv()),
            ["", "config.arm64_v8a", "config.xhdpi", "config.en", "config.nl"]
        );
    }

    #[test]
    fn a_32_bit_tv_gets_the_32_bit_part_even_with_a_64_bit_one_there() {
        let mut profile = tv();
        profile.abis = vec!["armeabi-v7a".into(), "armeabi".into()];
        assert_eq!(
            picked(&["", "config.arm64_v8a", "config.armeabi_v7a"], &profile),
            ["", "config.armeabi_v7a"]
        );
    }

    #[test]
    fn says_so_when_nothing_suits_the_processor() {
        let mut profile = tv();
        profile.abis = vec!["armeabi-v7a".into()];
        let error = choose(&[None, Some("config.arm64_v8a".into())], &profile).unwrap_err();
        assert!(error.contains("armeabi-v7a"), "{error}");
    }

    #[test]
    fn takes_the_nearest_density_up_and_the_highest_when_none_is_high_enough() {
        let mut profile = tv();
        profile.density = 213;
        assert_eq!(
            picked(&["", "config.mdpi", "config.xhdpi", "config.xxhdpi"], &profile),
            ["", "config.xhdpi"]
        );
        profile.density = 640;
        assert_eq!(
            picked(&["", "config.mdpi", "config.xhdpi"], &profile),
            ["", "config.xhdpi"]
        );
    }

    #[test]
    fn keeps_every_language_when_none_matches_and_english_is_missing() {
        assert_eq!(
            picked(&["", "config.fr", "config.de"], &tv()),
            ["", "config.fr", "config.de"]
        );
    }

    #[test]
    fn a_feature_picks_its_own_parts_and_stays_in() {
        assert_eq!(
            picked(
                &[
                    "",
                    "config.xhdpi",
                    "camera",
                    "camera.config.arm64_v8a",
                    "camera.config.x86",
                    "camera.config.xhdpi",
                    "camera.config.mdpi",
                ],
                &tv()
            ),
            ["", "config.xhdpi", "camera", "camera.config.arm64_v8a", "camera.config.xhdpi"]
        );
    }

    #[test]
    fn keeps_what_it_does_not_recognise() {
        assert_eq!(
            picked(&["", "config.test", "config.anydpi", "assetpack"], &tv()),
            ["", "config.test", "config.anydpi", "assetpack"]
        );
    }

    #[test]
    fn keeps_every_density_when_the_tv_did_not_say() {
        let mut profile = tv();
        profile.density = 0;
        assert_eq!(
            picked(&["", "config.mdpi", "config.xhdpi"], &profile),
            ["", "config.mdpi", "config.xhdpi"]
        );
    }

    #[test]
    fn stops_unpacking_at_the_limit_whatever_the_archive_claims() {
        use std::io::Write;
        let mut raw = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(io::Cursor::new(&mut raw));
            zip.start_file(
                "bomb.apk",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
            zip.write_all(&vec![0u8; 1024 * 1024]).unwrap();
            zip.finish().unwrap();
        }
        // Rewrites the central directory to say this unpacks to 10 bytes, as a real bomb would.
        let central = raw.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
        raw[central + 24..central + 28].copy_from_slice(&10u32.to_le_bytes());
        let mut archive = ZipArchive::new(io::Cursor::new(raw)).unwrap();
        assert_eq!(archive.by_index(0).unwrap().size(), 10);
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("out");
        let error = extract(&mut archive, 0, &path, 4096).unwrap_err();
        assert!(error.contains("too large"), "{error}");
        assert!(!path.exists());
    }

    #[test]
    fn reads_what_a_tv_reports() {
        let raw = "Physical density: 320\nOverride density: 240\n320\nlocales nl-NL,en-US,null,nl-NL,en-US\n";
        let profile = parse_profile(34, vec!["arm64-v8a".into()], raw);
        assert_eq!(profile.density, 240);
        assert_eq!(profile.languages, ["nl", "en"]);
        assert_eq!(parse_profile(28, vec![], "320\nlocales ,,\n").density, 320);
        assert_eq!(parse_profile(28, vec![], "Physical density: 213\n").density, 213);
        assert_eq!(parse_profile(28, vec![], "").density, 0);
    }

    #[test]
    fn leaves_whole_app_builds_out_of_a_split_set() {
        let names: Vec<String> = [
            "toc.pb",
            "splits/base-master.apk",
            "splits/base-xhdpi.apk",
            "standalones/standalone-arm64_v8a_xhdpi.apk",
            "instant/instant-base-master.apk",
            "universal.apk",
            "__MACOSX/splits/._base-master.apk",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(apk_entries(&names), [1, 2]);
        let universal = ["toc.pb", "universal.apk"].map(String::from).to_vec();
        assert_eq!(apk_entries(&universal), [1]);
    }

    #[test]
    fn game_data_names_cannot_reach_outside_their_folder() {
        assert_eq!(
            obb_name("Android/obb/com.game/main.12.com.game.obb"),
            Some("main.12.com.game.obb")
        );
        assert_eq!(obb_name("main.12.com.game.OBB"), Some("main.12.com.game.OBB"));
        for bad in ["Android/obb/x/.obb", "a/b;rm -rf.obb", "a/b c.obb", "a/b.obb/", "a/b.txt"] {
            assert_eq!(obb_name(bad), None, "{bad}");
        }
    }
}
