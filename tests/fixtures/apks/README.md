# Owned APK signing/install fixtures

These tiny APKs contain a manifest and a public test asset, with no activity, code or user data. They use package `org.bridgecoil.fixture.install`. `scripts/generate-apk-fixtures.py` builds them with Android SDK aapt2/zipalign/apksigner36.1.0 and temporary disposable signing keys. Private keys are never retained. Regeneration creates new fingerprints and updates fixtures.json.

The SDK independently verified all fixtures marked sdkVerified=true, and rejected the tampered APK after its asset bytes changed. The unsigned APK is deliberately unsigned. Current fixtures cover RSA/ECDSA v2, v2+v3, legacy v1, a same-signer version update, a config split and a different signer. They have not been installed on any device by this generator.

These fixtures establish local parser/cryptography behavior. They do not prove Android TV installation, OEM compatibility or controller pairing interoperability.
