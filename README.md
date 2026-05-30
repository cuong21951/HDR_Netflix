# HDR Netflix

Windows tray app that turns HDR on when Netflix is open and turns HDR off when Netflix closes.

## Easiest Install

1. Download `HDRNetflix.exe` from the latest GitHub Release.
2. Put it somewhere stable, for example:

   ```text
   %LOCALAPPDATA%\HDRNetflix\HDRNetflix.exe
   ```

3. Double-click `HDRNetflix.exe`.
4. Open the tray icon menu and enable `Start with Windows` if you want it to run at login.

You can also run the exe directly from Downloads, but if you enable `Start with Windows`, do not move or delete that exe afterward.

## Tray Menu

- `Enable/Disable auto HDR`: turns automatic Netflix detection on or off.
- `Enable/Disable start with Windows`: controls startup at login.
- `HDR displays`: chooses which display HDR should be toggled on.
- `Turn HDR on now`: manually turns HDR on for selected displays.
- `Turn HDR off now`: manually turns HDR off for selected displays.
- `Quit`: exits the tray app.

## Display Selection

By default, all active HDR-capable displays are controlled. To choose displays:

```text
HDR displays -> All displays
HDR displays -> <display name>
```

If `All displays` is checked, Netflix controls every active HDR-capable display. If one or more display names are checked, Netflix controls only those displays.

## Build From Source

```powershell
git clone <repo-url>
cd HDR_Netflix
cargo build --release
.\target\release\HDRNetflix.exe
```

To install your local build to `%LOCALAPPDATA%\HDRNetflix` and enable startup:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

To install without startup:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -NoStartup
```

## CLI Commands

```powershell
HDRNetflix.exe --status
HDRNetflix.exe --on
HDRNetflix.exe --off
HDRNetflix.exe --detect-netflix
```

## Config File

First run creates:

```text
%LOCALAPPDATA%\HDRNetflix\config.toml
```

You can also edit the config file manually. Run `HDRNetflix.exe --status`, copy the target key, and put it in `selected_targets`. Target keys are stable across reboots.

Example:

```toml
auto_enabled = true
poll_interval_ms = 1500
turn_off_when_netflix_closes = true
selected_targets = ["a1b2c3d4e5f60718"]
process_names = ["Netflix.exe"]
detect_window_titles = true
```

> Upgrading from a build before this fix? Display keys were previously derived
> from the adapter LUID, which Windows regenerates on every boot, so saved
> selections did not survive a restart. Re-pick your displays once from
> `HDR displays` in the tray menu (or clear `selected_targets` to control all
> HDR-capable displays). New keys are reboot-stable.

## Publish A Release

This repo includes a GitHub Actions workflow. To publish a public Windows exe:

```powershell
git tag v0.1.0
git push origin main --tags
```

The workflow builds `HDRNetflix.exe` on Windows and attaches it to the GitHub Release.

## Uninstall

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
```

## License

Licensed under the [MIT License](LICENSE).
