# DLSS5 AIO Injector — v0.1.0-alpha

First alpha release. One click turns a normal game install into a Neural Rendering + Frame Generation setup.

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
