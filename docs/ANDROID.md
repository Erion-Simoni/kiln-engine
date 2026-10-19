# Android Export Guide

This guide explains how to build and export games for Android using Kiln.

## Prerequisites

### Required Tools

1. **Rust toolchain** with Android targets:
   ```bash
   rustup target add aarch64-linux-android
   rustup target add armv7-linux-androideabi
   ```

2. **cargo-apk** for APK/AAB generation:
   ```bash
   cargo install cargo-apk
   ```

3. **Android SDK** and **NDK**:
   - Install Android Studio or the command-line tools
   - Set `ANDROID_SDK_ROOT` environment variable
   - Ensure NDK is installed (Android Studio: SDK Manager -> SDK Tools -> NDK)

4. **ADB** (Android Debug Bridge) for device testing:
   - Included with Android SDK
   - Verify installation: `adb version`

## Building an APK

### Quick Build

Use the justfile recipe:
```bash
just android
```

This builds the APK for the `aarch64-linux-android` target (64-bit ARM).

### Manual Build

```bash
cargo apk build --release -p kiln-player
```

The APK will be located at:
```
target/aarch64-linux-android/release/apk/kiln-player.apk
```

## Installing on a Device

### Via ADB

1. Enable USB debugging on your Android device (Developer Options)
2. Connect device via USB
3. Install APK:
   ```bash
   adb install target/aarch64-linux-android/release/apk/kiln-player.apk
   ```

### Manual Installation

1. Copy the APK to your device
2. Enable "Install from unknown sources" in device settings
3. Open the APK file to install

## CLI Export

Use the kiln-cli export command for Android:

```bash
cargo run --bin kiln-cli -- export --platform android --output export
```

This creates an APK in the `export` directory.

## Signed AAB for Play Store

To publish to Google Play Store, you need a signed Android App Bundle (AAB).

### 1. Generate a Keystore

```bash
keytool -genkey -v -keystore keystore.jks \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias upload
```

Store the keystore securely and never commit it to version control.

### 2. Configure Signing in Cargo.toml

Add to `kiln-player/Cargo.toml`:

```toml
[package.metadata.android.signing]
path = "../keystore.jks"
keystore_password = "your_password"
```

**Important:** For production, use environment variables or a secrets manager instead of hardcoding passwords.

### 3. Build Signed AAB

```bash
cargo-apk build --release -p kiln-player --signed
```

The AAB will be located at:
```
target/aarch64-linux-android/release/bundle/
```

### 4. Upload to Play Console

1. Go to Google Play Console
2. Create a new app or select existing
3. Upload the AAB file
4. Complete the store listing, content rating, and pricing
5. Submit for review

## Android-Specific Configuration

### App Metadata (kiln-player/Cargo.toml)

```toml
[package.metadata.android]
build_targets = ["aarch64-linux-android"]

[[package.metadata.android.uses_permission]]
name = "android.permission.INTERNET"

[package.metadata.android.application]
label = "Kiln Game"
```

### Permissions

Add required permissions as needed:

```toml
[[package.metadata.android.uses_permission]]
name = "android.permission.INTERNET"

[[package.metadata.android.uses_permission]]
name = "android.permission.WRITE_EXTERNAL_STORAGE"
```

### Supported Architectures

- `aarch64-linux-android` (64-bit ARM, primary target)
- `armv7-linux-androideabi` (32-bit ARM, legacy devices)

## Touch Input

Touch input is handled through winit's `WindowEvent::Touch` events. The `InputState` in `kiln-runtime` tracks active touches:

```rust
use kiln_runtime::InputState;

let touch_count = input_state.touch_count();
let primary_touch = input_state.get_primary_touch();
```

Touch events support:
- `TouchPhase::Started` - finger down
- `TouchPhase::Moved` - finger drag
- `TouchPhase::Ended` - finger up
- `TouchPhase::Cancelled` - system cancel

## Android Lifecycle

The Android player properly handles:

- **Resumed**: Window and GPU surface created
- **Suspended**: GPU surface dropped (required by Android)
- **Rotation**: Surface recreated automatically by winit

## Troubleshooting

### Build Errors

**Error: "ANDROID_SDK_ROOT not set"**
- Set the environment variable to your Android SDK path
- Example (Windows): `setx ANDROID_SDK_ROOT "C:\Users\YourName\AppData\Local\Android\Sdk"`

**Error: "NDK not found"**
- Install NDK via Android Studio: SDK Manager -> SDK Tools -> NDK (Side by Side)
- Ensure NDK version is compatible with cargo-apk

### Runtime Errors

**App crashes on startup**
- Check logcat: `adb logcat`
- Ensure GPU drivers support WebGPU/Vulkan
- Try Low render tier in project settings

**Touch input not working**
- Verify winit touch events are being handled in `android.rs`
- Check `InputState` touch tracking in game systems

## Size Budget

APK size budget: <20 MB (includes player binary + assets)

The Android player uses the same runtime crates as desktop, ensuring feature parity while maintaining size constraints through feature gating.

## CI/CD

For automated Android builds in CI:

1. Install Android SDK and NDK in CI environment
2. Set up signing keys via CI secrets
3. Run `cargo apk build --release -p kiln-player`
4. Upload APK/AAB to artifact storage or Play Store

See `docs/PACKAGING.md` for signing and CI integration details.
