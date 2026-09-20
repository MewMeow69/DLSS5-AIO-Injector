import { useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { Pill, SectionLabel } from "./ui";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4 border-b border-white/[0.045] py-1.5 last:border-0">
      <span className="w-[190px] shrink-0 text-[12px] font-medium">{label}</span>
      <span className="flex-1 text-[12px] text-deck-muted">{children}</span>
    </div>
  );
}

function Step({ n, title, children }: { n: number; title: string; children: React.ReactNode }) {
  return (
    <div className="flex gap-3 py-1.5">
      <span className="mt-[1px] grid h-5 w-5 shrink-0 place-items-center rounded-md bg-white/[0.07] text-[11px] font-bold text-deck-text/80">
        {n}
      </span>
      <div className="min-w-0">
        <div className="text-[12.5px] font-semibold">{title}</div>
        <div className="mt-0.5 text-[12px] leading-relaxed text-deck-muted">{children}</div>
      </div>
    </div>
  );
}

export function GuidePage() {
  const [copied, setCopied] = useState(false);
  const copyVulkan = () => {
    navigator.clipboard.writeText("-force-vulkan").catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
      <h1 className="text-[20px] font-bold tracking-normal">How to Use</h1>
      <div className="mt-1 text-[12px] font-medium text-deck-muted">
        Three clicks per game: pick it, press Install, launch and enable NR in the overlay.
      </div>

      <section className="mt-5">
        <SectionLabel>Getting Started</SectionLabel>
        <div className="tile px-4 py-2">
          <Step n={1} title="Let it scan">
            Steam, Epic, GOG and folders you add with <b>Add Folder</b>. Cards show the API, upscaler and frame-gen
            support it detected.
          </Step>
          <Step n={2} title="Pick the options and press Install">
            Everything missing is downloaded during install - OptiScaler or RenoDX, the NR runtime for your GPU, the
            feeder and ReShade if the game needs them, frame-generation runtimes. Watch the log if you are curious.
          </Step>
          <Step n={3} title="Launch the game and enable NR">
            <b>Insert</b> opens the OptiScaler overlay - tick <b>Neural Rendering</b> there. <b>Home</b> opens ReShade;
            its Add-ons tab should list <i>DLSS 5 Feed</i>. <b>Page Up/Down</b> cycles the stats overlay.
          </Step>
        </div>
      </section>

      <section className="mt-5">
        <SectionLabel>Which Neural Rendering Provider?</SectionLabel>
        <div className="tile px-4 py-2">
          <Row label="OptiScaler NR">
            Default. The game's upscaler call is replaced with DLSS and NR runs on it. Fastest path and works together
            with frame generation. Use it whenever the game has <b>DLSS, FSR2/3 or XeSS</b>.
          </Row>
          <Row label="RenoDX DLSS 5">
            NR runs as a ReShade add-on through the feeder instead of OptiScaler. Pick it for games OptiScaler cannot
            hook, or if you simply prefer the RenoDX image. The <b>performance</b> variant is the leanest; the
            Speedlemur build wants <b>HDR Transfer Strength = 0</b> if you see blobs in shadows.
          </Row>
          <Row label="No upscaler in the game">
            Nothing to hook, so the app installs ReShade plus the feeder, which supplies the motion vectors and depth
            NR needs. This happens automatically.
          </Row>
        </div>
      </section>

      <section className="mt-5">
        <SectionLabel>Which NR Runtime (picked from your GPU)?</SectionLabel>
        <div className="tile px-4 py-2">
          <Row label="RTX 50 · NVIDIA 310.8">
            The original signed runtime. Highest quality, made for Blackwell.
          </Row>
          <Row label="RTX 20/30/40 · ShortFuse">
            NVIDIA never shipped the model for these cards, so the cross-generation build is used. It works everywhere
            the original does, but the RTX 20/30 path costs more GPU time - that is why <b>Balanced</b> (67% model
            resolution) is the default there. On RTX 40 try <b>Quality</b> first.
          </Row>
          <Row label="AMD / Intel">
            NR needs NVIDIA's NGX core, so it cannot run. The app still installs OptiScaler so you can swap upscalers
            and use FSR FG or XeFG.
          </Row>
        </div>
      </section>

      <section className="mt-5">
        <SectionLabel>Frame Generation - Pick By What You Have</SectionLabel>
        <div className="tile px-4 py-2">
          <Row label="Game has DLSS Frame Gen">
            Use <b>DLSSG</b> with <b>Streamline</b> enabled. On RTX 20/30 add the <b>FG sm86</b> proxy so the DLSSG
            runtime runs on Ampere/Turing.
          </Row>
          <Row label="RTX 20/30/40 + DLSSG">
            The <b>MFG Unlock fork</b> (OptiScaler Build menu) raises the multiplier: up to 4X on Ampere/Turing, 6X on
            Ada. Restart the game after switching.
          </Row>
          <Row label="Any GPU, no DLSSG">
            <b>FSR FG</b> works everywhere; turn <b>HUDfix</b> on to avoid HUD ghosting. <b>XeFG</b> also runs on any
            GPU via the XeSS 3.x libraries the app installs - it needs Borderless Fullscreen, not exclusive.
          </Row>
          <Row label="Artur's DLSS Enabler">
            FSR multi-frame generation inside OptiScaler. Its author ships an installer, so download it from the game's
            frame-generation options, run it, then press Update / Repair.
          </Row>
          <Row label="No frame-gen support at all">
            OptiFG (FG Input = Upscaler) can synthesise frames in DX12 games. Expect more artefacts than a native
            implementation.
          </Row>
        </div>
      </section>

      <section className="mt-5">
        <SectionLabel>NR Presets</SectionLabel>
        <div className="tile px-4 py-2">
          <Row label="Ultra">100% model resolution, 2 passes. Sharpest, roughly 2x model cost.</Row>
          <Row label="Quality">100% model resolution, 1 pass. The default on RTX 40/50.</Row>
          <Row label="Balanced">67% model resolution, 1 pass. Default on RTX 20/30.</Row>
          <Row label="Performance">50% model resolution, 1 pass. Use it when the model eats too much frame time.</Row>
          <div className="pt-2 text-[11.5px] text-deck-muted">
            Model resolution only affects NR's own work - your game image keeps full detail. Tune it per game in the
            <b> Settings</b> tab and press <b>Save Settings</b>; nothing needs a reboot.
          </div>
        </div>
      </section>

      <section className="mt-5">
        <SectionLabel>When Something Is Off</SectionLabel>
        <div className="tile px-4 py-2">
          <Row label="No overlay">
            The game may not load that proxy name. Re-install and pick a different build/proxy, or try <b>Alt+Insert</b>.
          </Row>
          <Row label="Stuck on “waiting for upscaler”">
            NR has no input. Enable <b>Feeder</b> for that game (or use the RenoDX provider) and re-install.
          </Row>
          <Row label="Unity game crashes">
            The feeder's D3D11 path needs Vulkan in many Unity games.
            <button className="ml-2 underline" onClick={copyVulkan}>
              {copied ? "copied" : "copy -force-vulkan"}
            </button>{" "}
            into the game's Steam launch options.
          </Row>
          <Row label="Smearing or ghosting">
            ReShade → Add-ons → Generic Depth: select the scene depth, not a UI buffer. For FSR/XeFG frames, enable
            HUDfix.
          </Row>
          <Row label="Out of VRAM or low FPS">
            Lower Model Resolution first, then Generated Frames. RTX 20/30 users should stay at Balanced or
            Performance.
          </Row>
          <Row label="Anything broke">
            <b>Roll Back</b> on the game page restores the files from before the install. <b>Uninstall</b> returns the
            game to stock.
          </Row>
          <Row label="Logs">
            They sit next to the game executable: <code>OptiScaler.log</code>, <code>dlss5-feed.log</code>,
            <code>ReShade.log</code>.
          </Row>
        </div>
      </section>

      <section className="mt-5">
        <SectionLabel>Good To Know</SectionLabel>
        <div className="tile px-4 py-3 text-[12px] leading-relaxed text-deck-muted">
          <div className="flex flex-wrap gap-1.5 pb-2">
            <Pill tone="amber">Single-player only</Pill>
            <Pill tone="cyan">NR is off until you enable it</Pill>
            <Pill tone="violet">Nothing is bundled - everything is downloaded</Pill>
          </div>
          Injected DLLs can trip anti-cheat systems; never use this in online games. Updates for every component live on
          the <b>Components</b> page - the app checks on start and you can pin any version you prefer. Everything the
          app stores (downloads, registry, preferences) is listed in <b>Settings</b>.
          <div className="mt-2">
            <button
              className="btn btn-sm"
              onClick={() => openPath("https://github.com/MewMeow69/DLSS5-AIO-Injector#readme").catch(() => {})}
            >
              Full Documentation
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
