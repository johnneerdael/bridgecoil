use adb_core::bundle::{DeviceProfile, is_bundle, unpack};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/apks")
        .join(name)
}
fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixture(name)).unwrap()
}
fn zip_of(path: &Path, entries: &[(&str, Vec<u8>)]) {
    let mut zip = ZipWriter::new(std::fs::File::create(path).unwrap());
    for (name, data) in entries {
        zip.start_file(
            *name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}
fn tv() -> DeviceProfile {
    DeviceProfile {
        sdk: 34,
        abis: vec!["arm64-v8a".into()],
        density: 320,
        languages: vec!["en".into()],
    }
}
const PACKAGE: &str = "org.bridgecoil.fixture.install";

fn opened(entries: &[(&str, Vec<u8>)]) -> Result<adb_core::bundle::Unpacked, String> {
    let folder = tempfile::tempdir().unwrap();
    let archive = folder.path().join("app.bin");
    zip_of(&archive, entries);
    let out = tempfile::tempdir().unwrap();
    let result = unpack(&archive, out.path(), &tv());
    // The parts are only asked about, so their folder can go here.
    result
}

#[test]
fn every_layout_gives_the_base_first_and_its_split() {
    for (base, split) in [
        ("base.apk", "split_config.test.apk"),
        ("org.bridgecoil.fixture.install.apk", "config.test.apk"),
        ("splits/base-master.apk", "splits/base-test.apk"),
        ("base.APK", "nested/folder/split.apk"),
    ] {
        let set = opened(&[
            ("manifest.json", b"{}".to_vec()),
            (split, bytes("config-test.apk")),
            (base, bytes("v2-rsa.apk")),
        ])
        .unwrap_or_else(|e| panic!("{base}: {e}"));
        assert_eq!(set.set.package, PACKAGE);
        assert!(set.set.files[0].info.split.is_none());
        assert_eq!(set.set.files[1].info.split.as_deref(), Some("config.test"));
        assert_eq!(set.available, 2);
    }
}

#[test]
fn the_paths_stay_with_their_parts() {
    let folder = tempfile::tempdir().unwrap();
    let archive = folder.path().join("app.apks");
    zip_of(
        &archive,
        &[
            ("config.test.apk", bytes("config-test.apk")),
            ("base.apk", bytes("v2-rsa.apk")),
        ],
    );
    let out = tempfile::tempdir().unwrap();
    let set = unpack(&archive, out.path(), &tv()).unwrap();
    for (path, file) in set.paths.iter().zip(&set.set.files) {
        assert_eq!(
            adb_core::apk::inspect_package(path).unwrap().sha256,
            file.info.sha256
        );
        assert!(path.starts_with(out.path()));
    }
    assert_eq!(set.sha256.len(), 64);
}

#[test]
fn game_data_lands_in_the_folder_under_its_own_name() {
    let folder = tempfile::tempdir().unwrap();
    let archive = folder.path().join("game.xapk");
    zip_of(
        &archive,
        &[
            ("base.apk", bytes("v2-rsa.apk")),
            (
                "../../../../etc/Android/obb/com.other/main.3.org.bridgecoil.fixture.install.obb",
                vec![7; 2048],
            ),
        ],
    );
    let out = tempfile::tempdir().unwrap();
    let set = unpack(&archive, out.path(), &tv()).unwrap();
    assert_eq!(set.obb.len(), 1);
    assert_eq!(set.obb[0].name, "main.3.org.bridgecoil.fixture.install.obb");
    assert_eq!(set.obb[0].bytes, 2048);
    assert!(set.obb[0].path.starts_with(out.path()));
    assert_eq!(std::fs::read(&set.obb[0].path).unwrap(), vec![7; 2048]);
}

#[test]
fn a_game_data_name_that_is_not_plain_is_refused() {
    let error = opened(&[
        ("base.apk", bytes("v2-rsa.apk")),
        ("bad; name.obb", vec![1]),
    ])
    .unwrap_err();
    assert!(error.contains("game data"), "{error}");
}

#[test]
fn says_what_is_wrong_with_files_it_cannot_install() {
    for (entries, expected) in [
        (vec![("readme.txt", b"hi".to_vec())], "no APK inside"),
        (
            vec![("config.test.apk", bytes("config-test.apk"))],
            "only part of an app",
        ),
        (
            vec![
                ("a.apk", bytes("v2-rsa.apk")),
                ("b.apk", bytes("update-v2.apk")),
            ],
            "separate APKs",
        ),
        (
            vec![
                ("base.apk", bytes("v2-rsa.apk")),
                ("broken.apk", b"not a zip".to_vec()),
            ],
            "can't be read",
        ),
        (
            vec![
                ("base.apk", bytes("v2-rsa.apk")),
                ("config.test.apk", bytes("other-signer.apk")),
            ],
            "",
        ),
    ] {
        let error = opened(&entries).unwrap_err();
        assert!(error.contains(expected), "{expected}: {error}");
    }
}

#[test]
fn a_part_signed_by_someone_else_is_refused() {
    let split = bytes("config-test.apk");
    let tampered = {
        let mut changed = split.clone();
        let middle = changed.len() / 2;
        changed[middle] ^= 0xff;
        changed
    };
    assert!(
        opened(&[
            ("base.apk", bytes("v2-rsa.apk")),
            ("config.test.apk", tampered),
        ])
        .is_err()
    );
}

#[test]
fn tells_an_apk_from_a_bundle_by_its_contents() {
    let folder = tempfile::tempdir().unwrap();
    assert!(!is_bundle(&fixture("v2-rsa.apk")).unwrap());
    let zipped = folder.path().join("named.apk");
    zip_of(&zipped, &[("base.apk", bytes("v2-rsa.apk"))]);
    assert!(is_bundle(&zipped).unwrap());
    let store = folder.path().join("store.aab");
    zip_of(
        &store,
        &[
            ("BundleConfig.pb", vec![0]),
            ("base/manifest/AndroidManifest.xml", vec![0]),
        ],
    );
    assert!(is_bundle(&store).unwrap_err().contains(".aab"));
    let text = folder.path().join("notes.xapk");
    std::fs::write(&text, "hello").unwrap();
    assert!(is_bundle(&text).unwrap_err().contains("can read"));
}

#[test]
fn a_password_protected_file_is_named_as_that() {
    let folder = tempfile::tempdir().unwrap();
    let archive = folder.path().join("locked.apkm");
    let mut raw = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut raw));
        zip.start_file(
            "base.apk",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(b"secret").unwrap();
        zip.finish().unwrap();
    }
    // Sets the "encrypted" bit in both headers, which is how such files mark themselves.
    let local = raw.windows(4).position(|w| w == b"PK\x03\x04").unwrap();
    raw[local + 6] |= 1;
    let central = raw.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
    raw[central + 8] |= 1;
    std::fs::write(&archive, raw).unwrap();
    let out = tempfile::tempdir().unwrap();
    let error = unpack(&archive, out.path(), &tv()).unwrap_err();
    assert!(error.contains("password"), "{error}");
}
