# DLSS5 AIO Injector — v0.1.2-alpha

Third alpha. One click turns a normal game install into a Neural Rendering + Frame Generation setup.

## New in 0.1.2

- **How to Use page** — a short in-app guide: the 3-step flow, when to pick OptiScaler NR vs the RenoDX path, which NR
  runtime your GPU needs (and why RTX 20/30 uses the ShortFuse build with a lower model resolution), which frame
  generation to pick for what a game already has, presets, overlay keys and fixes for the usual problems.
- **Settings page** — default NR preset, "leave NR disabled after install", pre-release channel, component update checks
  on start, optional GitHub token (removes API rate-limit warnings), storage breakdown with a download-cache clearer,
  manually added game folders, app update check and a data reset.
- **DLSS preset control** per game — force a DLSS render preset (D–K) for every quality mode, plus the generic NGX app-id
  fix for games where overrides do not apply.
- **App update banner** — the start-up check now looks at this repository's releases and offers the new version.
- **Verify Install** — after installing, the game panel's install tab checks the folder against what was selected
  (proxy DLL, `OptiScaler.ini`, runtime folders, NR/DLSS/DLSSD runtimes, XeFG libraries, sm86 config **and** its
  `version.dll` proxy, feeder add-on, ReShade host, rollback snapshot) and reads the feeder/OptiScaler logs for the
  usual startup problems.
- **Add Folder now takes a games folder, not just a game** — adding `E:\gameria` (or any parent) lists the games inside it,
  one entry per game; adding a single game's own folder still works and keeps its name. Games with no exe at their top
  level (Unreal titles like Stellar Blade, Hades II) are found through their real game exe, not the biggest helper exe.
- **Pre-existing mods are understood, not guessed at** — detection now covers every proxy name ReShade and OptiScaler
  can take, plus hand-installed `OptiScaler.asi`/`OptiScaler.dll` add-ons, DXVK, dgVoodoo2, DLSS Enabler (`nvngx.dll`)
  and fakenvapi. A hand-installed OptiScaler is updated **in place** on its own proxy name with the old files backed up,
  its `OptiScaler.ini` is kept, a hand-installed `.asi` copy is disabled and restored by Roll Back, an existing ReShade
  host is upgraded by the feeder installer with its ini merged, and existing sm86/NR/feeder files are replaced in place.
  DXVK, dgVoodoo2 and DLSS Enabler are reported as conflicts in the install plan instead of being silently overwritten.
- Fixed: OptiScaler's own binaries were misread as ReShade/sm86/DXVK installs (its strings mention them), and our own
  `OptiScaler\dlss-enabler-headless.dll` payload was misread as a DLSS Enabler conflict.
- Fixed: the neural consumer the feeder installer placed ignored the build you picked and always fetched the
  Dagherbou OptiScaler-DLSSNR release (≈130 MB). The chosen build's zip is now handed to the installer.
- Fixed: the per-game DLSS preset was overwritten by the global default preset on every re-open.
- Stale extraction folders are cleaned up automatically; installed games are never touched by any of the settings.

## New in 0.1.1

- **XeFG on RTX and AMD** — the app now installs `libxess_fg.dll` + `libxell.dll` from the Intel XeSS 3.x SDK and
  `fakenvapi.dll` (Reflex → XeLL) automatically when you pick XeFG output. Present it in Borderless Fullscreen.
- **MFG unlock fork** — [evairx/OptiScaler-MFG](https://github.com/evairx/OptiScaler-MFG) is now a selectable build:
  NVIDIA Multi Frame Generation on RTX 20/30/40 (4X Ampere/Turing, 6X Ada) with XeFG kept as an explicit output.
- **Smarter proxy choice** — `dxgi.dll` is used for OptiScaler whenever the name is free; ReShade only takes it when
  the feeder is actually installed.
- **ReShade only when it is needed** — a game that already has DLSS/FSR/XeSS gets no ReShade and no feeder unless you
  ask for them (or pick the RenoDX path, which requires them).

## Highlights

- **Automatic game discovery** — Steam (all libraries), Epic, GOG and manual folders, each detected for API (DX11/DX12/
  Vulkan), engine (Unity/Unreal), upscaler (DLSS/FSR/XeSS), frame generation and anti-cheat presence.
- **Two Neural Rendering paths** — OptiScaler NR (wilsjo2 / janblade / upstream nightly builds) or RenoDX DLSS 5 through
  the DLSS 5 Feeder, chosen per game.
- **Nothing bundled, everything fetched** — OptiScaler builds, NVIDIA NGX runtimes (original for RTX 50, ShortFuse
  cross-generation for RTX 20/30/40), DLSS 5 Feeder, ReShade add-on build, LumeniteFX, Streamline SDK, Intel XeSS SDK,
  OptiPatcher and the DLSSG sm86 proxy are all downloaded from their official sources during install, with progress.
- **Frame generation** — DLSSG via Streamline, FSR FG, XeFG on any GPU (XeSS 3.x libraries), Artur's DLSS Enabler for
  FSR MFG, and DLSSG on RTX 20/30 through the sm86 proxy.
- **Per-game settings tab** — NR placement, model resolution, pass count, strengths, FG input/output/replacement,
  ReShade motion-vector provider and mask tuning, all editable before launching the game.
- **Safe by default** — every install is journalled; one-click **Roll Back** restores the previous files, and NR stays
  disabled until you enable it in the OptiScaler overlay.
- **Components page** — per-fork build pools with version pins, file dates, SHA-256, stable/beta/nightly channels,
  filters, search and startup update checks.

## Install

Download `DLSS5 AIO Injector_0.1.2_x64-setup.exe` below and run it. It installs per-user (no administrator prompt) and
bootstraps the WebView2 runtime automatically if it is missing — no other prerequisites.

## Known limitations (alpha)

- Windows 10/11 x64 only.
- Neural Rendering needs an NVIDIA RTX card; AMD/Intel GPUs get frame generation and upscaler swapping.
- **Single-player only** — injected DLLs can trip anti-cheat systems.
- DLSS Enabler is a manual step (its author only ships an installer); the app downloads it, tells you what to do and
  picks the DLL up afterwards.
- XeFG presents in Borderless Fullscreen only.
- Xbox / Game Pass and launcher-wrapped titles are detection-only in this build.

Full details, credits and troubleshooting: see the [README](https://github.com/MewMeow69/DLSS5-AIO-Injector#readme).
