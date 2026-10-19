# Kiln Game Packaging and Signing Guide

This document explains how to package and sign games exported from Kiln for distribution on Windows, macOS, and Linux.

---

## Overview

Kiln games can be exported with `kiln-cli export --platform <platform> --packaging`. The `--packaging` flag creates platform-specific packages:

- **Windows**: ZIP archive (portable distribution)
- **macOS**: .app bundle (application bundle)
- **Linux**: AppDir structure (can be converted to AppImage with appimagetool)

For production releases, you should code-sign your games to avoid OS warnings and security prompts.

---

## Windows Code Signing

### Unsigned Builds

When you export a Windows game without code signing, users may see a Windows SmartScreen warning when they first run the executable:

> "Windows protected your PC"
> "Microsoft Defender SmartScreen prevented an unrecognized app from starting"

Users can click "More info" → "Run anyway" to proceed, but this is a poor user experience.

### Code Signing Certificate

To code-sign Windows executables, you need a **Code Signing Certificate** from a trusted Certificate Authority (CA):

**Recommended CAs:**
- DigiCert
- Sectigo (formerly Comodo)
- GlobalSign
- SSL.com

**Certificate types:**
- **OV (Organization Validated)**: $100-300/year, requires business verification
- **EV (Extended Validation)**: $200-500/year, stronger verification, better trust

**Free alternatives (for testing only):**
- Self-signed certificates (NOT recommended for distribution - users will still see warnings)
- **Warning**: Self-signed certificates do not bypass SmartScreen

### Getting a Certificate

1. Choose a CA and purchase an OV or EV code signing certificate
2. Complete the verification process (business documents, etc.)
3. Receive the certificate file (usually .pfx or .p12 format)
4. Store the certificate and password securely

### Signing Tools

Windows code signing uses the **SignTool** utility:

**Via Windows SDK:**
- Included with Windows 10/11 SDK
- Path: `C:\Program Files (x86)\Windows Kits\10\bin\<version>\x64\signtool.exe`

**Via Rust tools:**
- `cargo install sign-tool` (cross-platform wrapper)
- `oxisign` crate for programmatic signing

### Signing a Windows Executable

Using SignTool:
```bash
signtool sign \
  /f certificate.pfx \
  /p YOUR_PASSWORD \
  /tr http://timestamp.digicert.com \
  /td sha256 \
  /fd sha256 \
  game.exe
```

Parameters:
- `/f`: Certificate file
- `/p`: Certificate password
- `/tr`: Timestamp server URL (required for long-term validity)
- `/td`: Timestamp digest algorithm (sha256)
- `/fd`: File digest algorithm (sha256)
- The executable path

### Code Signing in CI

Store your certificate as a GitHub Actions secret:

1. Base64-encode the certificate:
   ```bash
   base64 -w 0 certificate.pfx > cert-base64.txt
   ```

2. Add to GitHub repository secrets:
   - `WINDOWS_CERTIFICATE`: Base64-encoded certificate
   - `WINDOWS_CERTIFICATE_PASSWORD`: Certificate password

3. In CI workflow:
   ```yaml
   - name: Decode certificate
     run: |
       echo "${{ secrets.WINDOWS_CERTIFICATE }}" | base64 -d > certificate.pfx

   - name: Sign executable
     run: |
       signtool sign /f certificate.pfx /p "${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}" \
         /tr http://timestamp.digicert.com /td sha256 /fd sha256 game.exe
   ```

### Verifying a Signature

```bash
signtool verify /pa /v game.exe
```

---

## macOS Code Signing and Notarization

### Unsigned Builds

When you export a macOS game without code signing, users may see a Gatekeeper warning:

> "Game.app cannot be opened because the developer cannot be verified"

Users can right-click → Open → Open to bypass, but this is a poor user experience.

### Apple Developer Account

To code-sign and notarize macOS apps, you need an **Apple Developer Program membership**:

- **Individual**: $99/year
- **Organization**: $99/year (recommended for distribution)

Benefits:
- Code signing certificates
- App Store distribution (if desired)
- Notarization service (required for macOS 10.15+)

### Creating a Signing Certificate

1. Enroll in Apple Developer Program
2. In Xcode → Preferences → Accounts, add your Apple ID
3. Request a **Developer ID Application** certificate:
   - Open Keychain Access
   - Certificate Assistant → Request a Certificate From a Certificate Authority
   - Upload CSR to Apple Developer portal
   - Download and install the certificate

### Code Signing a macOS App

Using `codesign` (built into macOS):
```bash
codesign --force --deep --sign "Developer ID Application: Your Name (TEAM_ID)" Game.app
```

Or using your certificate identity:
```bash
codesign --force --deep --sign "Developer ID Application" Game.app
```

### Notarization

**Notarization is required for macOS 10.15 (Catalina) and later.** Without notarization, Gatekeeper will block the app.

**Notarization process:**

1. **Create an app-specific password** for your Apple ID:
   - appleid.apple.com → Sign In → Security → App-Specific Passwords

2. **Notarize the app**:
   ```bash
   xcrun notarytool submit Game.app \
     --apple-id "your@email.com" \
     --password "app-specific-password" \
     --team-id "TEAM_ID" \
     --wait
   ```

3. **Staple the notarization ticket**:
   ```bash
   xcrun stapler staple Game.app
   ```

### Code Signing in CI

Store credentials as GitHub Actions secrets:

1. Add to repository secrets:
   - `APPLE_ID`: Your Apple ID email
   - `APPLE_APP_SPECIFIC_PASSWORD`: App-specific password
   - `APPLE_TEAM_ID`: Your 10-character team ID

2. In CI workflow (macOS runner):
   ```yaml
   - name: Code sign
     run: |
       codesign --force --deep --sign "Developer ID Application: ${{ secrets.APPLE_TEAM_ID }}" Game.app

   - name: Notarize
     run: |
       xcrun notarytool submit Game.app \
         --apple-id "${{ secrets.APPLE_ID }}" \
         --password "${{ secrets.APPLE_APP_SPECIFIC_PASSWORD }}" \
         --team-id "${{ secrets.APPLE_TEAM_ID }}" \
         --wait

   - name: Staple
     run: |
       xcrun stapler staple Game.app
   ```

### Verifying a Signature

```bash
codesign --verify --verbose Game.app
spctl -a -t exec -vv Game.app
```

---

## Linux Packaging

### AppImage

Linux does not require code signing in the same way as Windows/macOS. The primary packaging format is **AppImage**, which is a self-contained, executable file that runs on most Linux distributions.

**Creating an AppImage:**

1. Install `appimagetool`:
   ```bash
   wget https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
   chmod +x appimagetool-x86_64.AppImage
   ```

2. Convert AppDir to AppImage:
   ```bash
   ./appimagetool-x86_64.AppImage Game.AppDir Game-x86_64.AppImage
   ```

3. Make executable:
   ```bash
   chmod +x Game-x86_64.AppImage
   ```

**Note:** The current `kiln-cli export --packaging` implementation creates the AppDir structure. You must manually run `appimagetool` to create the final .AppImage file.

### Flatpak (Optional)

Flatpak provides sandboxing and centralized distribution via Flathub. To create a Flatpak:

1. Create a Flatpak manifest (`com.kiln.game.json`)
2. Use `flatpak-builder` to build
3. Submit to Flathub (requires Flathub maintainer approval)

**Note:** Flatpak support is deferred to a future milestone (see ADR-0018).

### tar.gz Fallback

For users who prefer manual extraction, you can also provide a simple tar.gz archive:

```bash
tar czf game-linux.tar.gz Game.AppDir
```

---

## Summary of Unsigned Build Warnings

| Platform | Warning | Bypass Method | Resolution |
|----------|---------|---------------|------------|
| Windows | SmartScreen "unrecognized app" | More info → Run anyway | Code sign with OV/EV certificate |
| macOS | Gatekeeper "developer cannot be verified" | Right-click → Open → Open | Code sign + notarize with Apple Developer account |
| Linux | None (optional executable flag) | `chmod +x` | None required (AppImage is self-contained) |

---

## Next Steps

1. **Testing**: Run `kiln-cli export --platform windows --packaging` locally to test packaging
2. **Certification**: Obtain code signing certificates for production releases
3. **CI Integration**: Add signing steps to GitHub Actions workflow
4. **User Documentation**: Document the signing process for game developers using Kiln

---

## References

- [Microsoft Code Signing Best Practices](https://docs.microsoft.com/en-us/windows/win32/seccrypto/cryptography-tools)
- [Apple Code Signing Guide](https://developer.apple.com/support/code-signing/)
- [Apple Notarization Guide](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [AppImage Documentation](https://docs.appimage.org/)

---

## Storefront Distribution

### itch.io

**itch.io** is the recommended primary storefront for Kiln games due to its simple publishing process and no upfront fees.

**Account setup:**
1. Create account at https://itch.io/
2. Create a new game page (My Games → Create new game)
3. Set up game metadata (title, description, screenshots, tags)

**Publishing with butler CLI:**
```bash
# Install butler from https://itch.io/docs/butler/
butler login
butler push export myusername/mygame:windows
butler push export myusername/mygame:macos
butler push export myusername/mygame:linux
```

**Manual upload:**
1. Go to your game page on itch.io
2. Click "Upload new version"
3. Upload the generated ZIP/.app/.AppImage files
4. Set platform (Windows/macOS/Linux) and channel (default)

**Fees:**
- Revenue share: 10% of sales
- No upfront fee
- No code signing required

**Review process:**
- No manual review; games are published immediately
- Content guidelines apply (no malware, illegal content, etc.)

### Steam

**Steam** provides access to a large audience and advanced features (achievements, cloud saves, leaderboards) via Steamworks.

**Account setup:**
1. Join Steamworks ($100 one-time fee)
2. Complete paperwork and tax forms
3. Create app entry in Steamworks partner site
4. Configure store page (metadata, screenshots, pricing)

**Publishing with steamcmd:**
```bash
# Install Steamworks SDK from Steamworks partner site
# Edit generated steam_build.vdf with your app ID and depot ID
steamcmd +login <username> +run_app_build steam_build.vdf +quit
```

**VDF template (generated by kiln-cli):**
```vdf
"appbuild"
{
    "appid" "YOUR_APP_ID"
    "desc" "My Game Build"
    "buildoutput" "./build_output"
    "contentroot" "./export"
    "setlive" "default"

    "depots"
    {
        "YOUR_DEPOT_ID"
        {
            "FileMapping"
            {
                "LocalPath" "*"
                "DepotPath" "."
                "recursive" "1"
            }
        }
    }
}
```

**Fees:**
- Revenue share: 30% of sales
- One-time fee: $100 (recouped after $1,000 in sales)
- Code signing required (see Windows/macOS sections above)

**Review process:**
- Store page review: 1-3 business days
- Build review: 1-5 business days
- Content guidelines apply (no illegal content, must be functional, etc.)

### Steam Deck / Linux Handheld Verification

**Steam Deck requirements:**
- Game must run on SteamOS 3 (Arch Linux-based)
- Controller support required (Steam Deck controls or external gamepad)
- 1280x800 resolution support (native Steam Deck resolution)
- Text must be readable at handheld distance
- Game must handle sleep/resume gracefully
- Performance: target 30-60 FPS on Steam Deck hardware

**Testing checklist:**
- [ ] Game launches from Steam in Gaming Mode
- [ ] Controller input works (D-pad, analog sticks, face buttons, triggers)
- [ ] UI is readable at 1280x800 resolution
- [ ] Performance is acceptable on Steam Deck hardware
- [ ] Game survives sleep/wake cycle (suspend/resume)
- [ ] No fullscreen-exclusive rendering (causes issues on Deck)
- [ ] Cloud saves work (if using Steamworks cloud save)

**Linux handhelds (other than Steam Deck):**
- ASUS ROG Ally, Lenovo Legion Go, AYANEO, etc.
- Similar requirements to Steam Deck
- Test on target hardware if possible
- Consider adding controller remapping options

**Kiln integration:**
- Use `kiln-platform` crate with `steam` feature for Steamworks integration
- Test on Steam Deck (or similar Linux handheld) before publishing
- Document controller support in store page
- Consider adding graphics quality presets for low-end handhelds
