# DLSS5 AIO Injector

**One click turns a normal game install into a Neural Rendering + Frame Generation setup.**
The app finds your games, works out what they already have (DLSS / FSR / XeSS, frame generation, which GPU you own),
and installs the complete stack for the right path — OptiScaler, the NVIDIA Neural Rendering runtime, the DLSS 5
Feeder, ReShade and the frame-generation runtimes — with per-game settings you can tune before you ever launch the game.

Nothing is bundled with the app. Every piece is downloaded from its official source while you watch the progress bar,
so there is no "download this zip, extract it there" step. Click **Install**, wait, launch your game.

---

## Why use it

Setting up Neural Rendering by hand is a 10-step ritual: pick the right OptiScaler fork build, choose a proxy DLL the
game will actually load, supply the correct `nvngx_dlssnr.dll` for your GPU generation, add the feeder and ReShade if
the game has no upscaler, wire the motion-vector provider, patch the INI, and hope you did not mix up the 20 case-
sensitive keys. This app does all of it, per game, and keeps a rollback snapshot of every file it touches.

| Without this app | With this app |
| --- | --- |
| Find the right OptiScaler fork + version | Picks from every fork (wilsjo2 / janblade / upstream nightly) |
| Guess which proxy DLL the game loads | Probes the game folder and allocates proxies across all mods |
| Know your GPU generation's NR runtime | Reads your GPU and fetches the matching runtime |
| Configure feeder + ReShade by hand | Runs the feeder installer with the correct consumer |
| Edit 60 KB of INI keys | Per-game settings panel with sliders and toggles |
| Broken game after a bad install | One-click Roll Back to the pre-install state |

## What it can install, per game

- **Neural Rendering (NR)** — two interchangeable paths:
  - **OptiScaler NR** (fork builds, proxy + INI + runtime) for games with DLSS/FSR/XeSS inputs
  - **RenoDX DLSS 5** through the DLSS 5 Feeder for games without them
- **NVIDIA runtimes** — `nvngx_dlss.dll`, `nvngx_dlssnr.dll` (original for RTX 50, ShortFuse cross-generation build for
  RTX 20/30/40), `nvngx_dlssd.dll` (Ray Reconstruction), `nvngx_dlssg.dll`
- **DLSS 5 Feeder + ReShade** — add-on build, DLSS5_Feed shader, LumeniteFX motion-vector provider, Vulkan layer
- **Frame generation** — DLSSG via Streamline, FSR FG, **XeFG on any GPU** (Intel XeSS 3.x libraries), Artur's
  DLSS Enabler for FSR MFG, **DLSSG on RTX 20/30** via the sm86 proxy
- **OptiPatcher** — unlocks DLSS/DLSSG inputs without spoofing in supported games

## Hardware support

| GPU | Neural Rendering | Frame generation |
| --- | --- | --- |
| RTX 50 (Blackwell) | NVIDIA-signed 310.8 runtime | native DLSSG / MFG |
| RTX 40 (Ada) | ShortFuse compatibility runtime | native DLSSG + optional MFG unlock build |
| RTX 30 / 20 (Ampere / Turing) | ShortFuse compatibility runtime | DLSSG via the sm86 proxy, FSR FG, XeFG |
| AMD / Intel | not possible (NR needs NVIDIA NGX) | FSR FG, XeFG, OptiScaler upscaler swap |

## Requirements

- Windows 10 / 11, 64-bit
- Internet connection (everything is downloaded on install)
- NVIDIA driver with NGX support for Neural Rendering; any DX12-capable GPU for frame generation
- **Not for online games.** Anti-cheat systems may flag injected DLLs. Single-player only.

## Install

1. Download `DLSS5-AIO-Injector_0.1.0_x64-setup.exe` from the [latest pre-release](../../releases).
2. Run it. It installs per-user (no administrator prompt), with Start Menu and optional desktop shortcuts.
3. The installer bootstraps the WebView2 runtime if it is missing — there is no separate prerequisite to install.

## Use

1. The app scans your Steam / Epic / GOG libraries on first start (use **Add Folder** for anything else).
2. Pick a game. The detail panel shows what was detected: API, engine, upscaler, frame generation, installed mods.
3. Choose your options — NR provider (OptiScaler or RenoDX), presets, frame-generation output, extras.
4. Press **Install** and watch the log. Everything missing is downloaded; the panel reports each stage.
5. Launch the game, press **Insert** for the OptiScaler overlay and enable **Neural Rendering** (it is off by default).
   Press **Home** for ReShade — the Add-ons tab should list *DLSS 5 Feed*.
6. Tune anything later in the game's **Settings** tab (NR, frame generation, ReShade feed) without launching the game.

Roll back any time: the game page keeps your pre-install files in `_NeuroDeck/backups` and **Roll Back** restores them.

## Components & updates

Every component (OptiScaler builds, feeder, ReShade, RenoDX, runtimes, XeSS SDK, OptiPatcher, sm86) has its own card:
latest stable version with its release date, every local copy with its file date and SHA-256, an **Active** pin to
choose which version installs, and a **Pre-Releases** switch for beta channels. The app checks for new releases on
startup and from the **Check for Updates** button; nightlies stay in their own channel.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| No overlay | Check the proxy DLL the game loads, try `Alt+Insert`; some games need `version.dll` instead of `winmm.dll` |
| "Waiting for the upscaler to run" | NR needs an upscaler input — enable the feeder for that game, or use the RenoDX path |
| Unity game crashes on launch | Add `-force-vulkan` to the launch options (the app shows a copy button for Unity titles) |
| Smearing / HDR blobs | ReShade → Add-ons → Generic Depth: pick the scene depth. For the RenoDX performance build set HDR Transfer Strength to 0 |
| XeFG does not appear | XeFG only presents in **Borderless Fullscreen**, and non-Intel GPUs need the XeSS 3.x libraries option |
| Out of VRAM | Lower Model Resolution (WorkingScale) in the game's Settings tab, e.g. 0.67 or 0.5 |
| Anything broke | **Roll Back** on the game page restores the previous files |

Logs live next to the game executable (`OptiScaler.log`, `dlss5-feed.log`, `ReShade.log`).

## Credits & attribution

This app is an installer/manager. All of the actual technology belongs to these projects, and none of it is bundled —
each piece is downloaded from its own source at install time.

- **[OptiScaler](https://github.com/optiscaler/OptiScaler)** (cdozdil and contributors, GPL-3.0) — the upscaler
  middleware this app installs and configures.
- **OptiScaler Neural Rendering forks** — [Dagherbou/OptiScaler_DLSSNR](https://github.com/Dagherbou/OptiScaler_DLSSNR),
  [wilsjo2/OptiScaler-DLSSNR-PreSR-Multipass](https://github.com/wilsjo2/OptiScaler-DLSSNR-PreSR-Multipass),
  [janblade](https://github.com/janblade/OptiScaler-DLSSNR-PreSR-Multipass) and the upstream
  [nightly builds](https://github.com/optiscaler/OptiScaler-nightly).
- **[RenoDX](https://github.com/clshortfuse/renodx) — ShortFuse** — the RenoDX DLSS add-on used for the alternative
  NR path, and the colour-composition design behind the Neural Rendering pass.
- **[DLSS5-Feeder](https://github.com/jlrouzies-fr/DLSS5-Feeder)** (Jean-Laurent Rouzies, MIT) — the feeder add-on that
  supplies motion vectors and depth, plus its installer script.
- **[ReShade](https://reshade.me)** (crosire, BSD-3-Clause) — the add-on host and Vulkan layer.
- **[LumeniteFX](https://github.com/umar-afzaal/LumeniteFX)** (umar-afzaal, MIT) — motion-vector provider.
- **[OptiPatcher](https://github.com/optiscaler/OptiPatcher)** (optiscaler) — DLSS/DLSSG input unlocking.
- **[dlssg_for_sm86](https://github.com/sdli1995/dlssg_for_sm86)** (sdli1995, GPL-3.0) — frame generation on RTX 20/30.
- **[DLSS Enabler](https://github.com/artur-graniszewski/DLSS-Enabler)** (Artur Graniszewski) — FSR frame generation /
  MFG inside OptiScaler. Its installer is downloaded and run by you.
- **NVIDIA** — `nvngx_dlss*.dll` and the Streamline SDK, downloaded from NVIDIA's public sources; never redistributed here.
- **Intel** — [XeSS SDK](https://github.com/intel/xess) (`libxess`, `libxess_fg`, `libxell`) — enables XeFG, including
  on non-Intel GPUs.
- **AMD** — FidelityFX/FSR runtimes, shipped inside the OptiScaler packages.
- **[rankFTW/rhi-repo](https://github.com/RankFTW/rhi-repo)** — public mirror used for the NGX runtimes and RenoDX builds.
- Built with [Tauri](https://tauri.app), [React](https://react.dev), [Tailwind CSS](https://tailwindcss.com) and
  [Roboto](https://fonts.google.com/specimen/Roboto) (Apache-2.0).

## License

**GPL-3.0** — see [LICENSE](LICENSE). The app is free software and integrates with GPL-3.0 projects (OptiScaler and its
Neural Rendering forks, dlssg_for_sm86); using the same license keeps the whole chain compatible. You may redistribute
and modify it under the same terms.

## Disclaimer

Unofficial project. Not affiliated with, endorsed by, or supported by NVIDIA, Intel, AMD, OptiScaler, RenoDX or any
game developer. Injecting DLLs into games can break anti-cheat protected titles — use it in single-player games only,
at your own risk. Everything is experimental: Neural Rendering is undocumented NVIDIA functionality driven directly by
the OptiScaler forks.
