# DLSS5 AIO Injector — v0.1.2-alpha

Third alpha. One click turns a normal game install into a Neural Rendering + Frame Generation setup.

## New in 0.1.2

- **Settings page** — default NR preset, "leave NR disabled after install", pre-release channel, component update checks
  on start, optional GitHub token (removes API rate-limit warnings), storage breakdown with a download-cache clearer,
  manually added game folders, app update check and a data reset.
- **DLSS preset control** per game — force a DLSS render preset (D–K) for every quality mode, plus the generic NGX app-id
  fix for games where overrides do not apply.
- **App update banner** — the start-up check now looks at this repository's releases and offers the new version.
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

Download `DLSS5-AIO-Injector_0.1.0_x64-setup.exe` below and run it. It installs per-user (no administrator prompt) and
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
