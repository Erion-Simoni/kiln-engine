# iOS Build Guide

This guide covers building, deploying, and submitting Kiln games to the iOS App Store.

## Prerequisites

### Required Tools

1. **macOS** (required for iOS development)
   - macOS 12 (Monterey) or later recommended
   - Apple Silicon (M1/M2/M3) preferred for faster builds

2. **Xcode** (from Mac App Store)
   ```bash
   xcode-select --install
   ```

3. **Rust iOS targets**
   ```bash
   # For device
   rustup target add aarch64-apple-ios

   # For simulator (Apple Silicon)
   rustup target add aarch64-apple-ios-sim

   # For simulator (Intel Macs)
   rustup target add x86_64-apple-ios
   ```

4. **Xcode Command Line Tools**
   ```bash
   xcode-select --install
   ```

### Verify Installation

```bash
# Check Xcode version
xcodebuild -version

# Check iOS SDK
xcrun --sdk iphoneos --show-sdk-path

# List available simulators
xcrun simctl list devices
```

## Building for iOS

### Quick Start

Run all steps in one go (on macOS):

```bash
# From project root
cargo build --release --target aarch64-apple-ios -p kiln-player
```

### Step-by-Step

#### 1. Build the Static Library

```bash
# For iOS device (ARM64)
cargo build --release --target aarch64-apple-ios -p kiln-player

# For iOS simulator (Apple Silicon)
cargo build --release --target aarch64-apple-ios-sim -p kiln-player
```

**Output:** `target/aarch64-apple-ios/release/libkiln_player.a`

#### 2. Create Xcode Project

iOS apps require an Xcode project for packaging, signing, and App Store submission. The static library (`libkiln_player.a`) is linked into an Objective-C app that calls `start_winit_app()`.

**Manual Xcode Project Setup:**

1. Create a new iOS App project in Xcode:
   - File → New → Project → iOS → App
   - Product Name: YourGame
   - Interface: Storyboard
   - Language: Objective-C

2. Add the static library to your Xcode project:
   - Drag `libkiln_player.a` into your project
   - Add to "Link Binary With Libraries" in Build Phases

3. Create a bridging header (`kiln_bridge.h`):
   ```objective-c
   #ifndef kiln_bridge_h
   #define kiln_bridge_h

   void start_winit_app(void);

   #endif /* kiln_bridge_h */
   ```

4. Modify `main.m` to call the Rust entry point:
   ```objective-c
   #import <UIKit/UIKit.h>
   #import "kiln_bridge.h"

   int main(int argc, char * argv[]) {
       @autoreleasepool {
           // Call the Rust winit app
           start_winit_app();
       }
       return 0;
   }
   ```

5. Configure Build Settings:
   - Architectures: arm64
   - Deployment Target: iOS 13.0 or later
   - Supported Platforms: iOS

#### 3. Configure Info.plist

Add required keys to `Info.plist`:

```xml
<key>CFBundleIdentifier</key>
<string>com.yourcompany.game</string>

<key>CFBundleVersion</key>
<string>1.0.0</string>

<key>CFBundleShortVersionString</key>
<string>1.0</string>

<key>UIRequiredDeviceCapabilities</key>
<array>
    <string>armv7</string>
</array>

<key>UILaunchScreen</key>
<dict>
    <key>UIImageName</key>
    <string>LaunchImage</string>
</dict>

<!-- Add these if your game uses sensitive features -->
<key>NSCameraUsageDescription</key>
<string>This game uses the camera for augmented reality features.</string>

<key>NSMicrophoneUsageDescription</key>
<string>This game uses the microphone for voice chat.</string>

<key>NSPhotoLibraryUsageDescription</key>
<string>This game accesses your photo library to save screenshots.</string>
```

#### 4. Add App Icons and Launch Screen

- Add app icons to Assets.xcassets (required sizes: 1024x1024, 180x180, 167x167, 120x120, 87x87, 80x80, 76x76, 60x60, 58x58, 40x40, 29x29, 20x20)
- Add launch screen image to Assets.xcassets
- Configure launch screen in Storyboard or via `UILaunchScreen` in Info.plist

## Code Signing and App Store Submission

### Apple Developer Account

1. **Enroll in Apple Developer Program** ($99/year)
   - https://developer.apple.com/programs/
   - Required for App Store distribution
   - Free account works for local testing only (7-day limit)

### Code Signing

1. **Create Signing Certificate**
   - In Xcode: Project → Signing & Capabilities
   - Select your development team
   - Xcode will automatically manage certificates

2. **Configure Provisioning Profile**
   - Development profile: for testing on devices
   - Distribution profile: for App Store submission
   - Xcode can automatically create profiles

3. **Enable Signing in Xcode**
   - Project → Signing & Capabilities
   - Check "Automatically manage signing"
   - Select your team and bundle identifier

### Building for App Store

1. **Select "Any iOS Device (arm64)"** as the destination
2. **Product → Archive** in Xcode
3. **Validate App** in the archive organizer
4. **Distribute App** → App Store Connect

### App Store Connect

1. **Create App Record**
   - App name, bundle ID, SKU, user info
   - Upload screenshots (required sizes)
   - Provide app description and metadata

2. **Upload Build**
   - From Xcode archive organizer
   - Or via Transporter app

3. **Submit for Review**
   - Complete all required fields
   - TestFlight beta testing (recommended)
   - Submit for App Review

## Testing on Device

### TestFlight

1. **Add Testers** in App Store Connect
2. **Create TestFlight Build** from your archive
3. **Invite Testers** via email or public link
4. **Testers install** via TestFlight app

### Local Device Testing

1. **Connect iOS device** to Mac via USB
2. **Trust device** on the device (Settings → General → Device Management)
3. **Select device** in Xcode's scheme selector
4. **Run** (⌘R) to deploy to device

## Testing on Simulator

```bash
# Boot a specific simulator
xcrun simctl boot "iPhone 15 Pro"

# Install the .app bundle
xcrun simctl install "iPhone 15 Pro" /path/to/YourGame.app

# Launch the app
xcrun simctl launch "iPhone 15 Pro" com.yourcompany.game

# View simulator logs
xcrun simctl spawn "iPhone 15 Pro" log stream --predicate 'process == "YourGame"'
```

## iOS-Specific Considerations

### App Lifecycle

iOS has strict lifecycle requirements. The winit event loop handles:
- `resumed`: Surface created, game runs
- `suspended`: Surface dropped, game pauses
- Loop may not receive exit event (SIGKILL on suspend)

### Device Tier Detection

Use `kiln_core::DeviceTier` for automatic tier selection:

```rust
use kiln_core::{DeviceTier, DeviceCapabilities, detect_device_tier};

let capabilities = DeviceCapabilities::default(); // Or actual device query
let tier = detect_device_tier(&capabilities);
```

**Tier thresholds (adjust based on testing):**
- **High**: 6GB+ RAM, 6+ cores, Apple/Adreno/Mali GPU
- **Medium**: 3-6GB RAM, 4-6 cores
- **Low**: <3GB RAM, <4 cores

### Orientation

Configure in project settings (`kiln.toml`):

```toml
[mobile]
orientation = "landscape"  # portrait, landscape, or auto
```

### Notch Safe Areas

For devices with notches (iPhone X and later):

```rust
// winit provides safe area insets via the window
// Ensure UI respects these insets
```

Configure in project settings:

```toml
[mobile.ios]
supports_notch = true
```

### Permissions

Declare permissions in project settings:

```toml
[mobile.ios]
permissions = ["NSCameraUsageDescription", "NSMicrophoneUsageDescription"]

[mobile.ios.privacy_descriptions]
camera = "This game uses the camera for AR features"
microphone = "This game uses the microphone for voice chat"
photo_library = "This game saves screenshots to your photo library"
```

## Size Budget

iOS App Store limit: **4 GB** (can be increased with approval)

Kiln target: **<25 MB** for player binary + assets

To measure size:

```bash
# Measure .app bundle size
du -sh YourGame.app

# Measure binary size
du -sh YourGame.app/YourGame
```

## Troubleshooting

### Build Errors

**"Linker command failed"**
- Ensure static library is built for correct target
- Check that `libkiln_player.a` is added to "Link Binary With Libraries"

**"Undefined symbol"**
- Verify `start_winit_app` is exported with `#[no_mangle]`
- Check that bridging header is included

**"Code signing error"**
- Verify Apple Developer account is active
- Check that bundle identifier is unique
- Ensure provisioning profile is valid

### Runtime Errors

**"Surface creation failed"**
- Ensure window is created in `resumed` event
- Check that wgpu backend is initialized correctly

**"GPU initialization failed"**
- Verify Metal is supported on device
- Check wgpu instance configuration

### App Store Rejection

**Common reasons:**
- Missing or incorrect metadata
- Violation of App Review guidelines
- Crashes on startup
- Missing privacy descriptions
- Inappropriate content

**Resources:**
- [App Store Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)
- [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/)

## CI/CD Integration

For automated iOS builds, use GitHub Actions with macOS runner:

```yaml
jobs:
  ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          target: aarch64-apple-ios
      - run: cargo build --release --target aarch64-apple-ios -p kiln-player
```

## References

- [winit iOS documentation](https://docs.rs/winit/latest/winit/platform/ios/index.html)
- [Apple Developer Documentation](https://developer.apple.com/documentation/)
- [App Store Connect Help](https://help.apple.com/app-store-connect/)
- [Rust iOS targets](https://doc.rust-lang.org/rustc/platform-support/apple-ios.html)
