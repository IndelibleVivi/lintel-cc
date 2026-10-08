# Local Preview film production

This is an independent, editable production toolchain for the Lintel concept
intro. It does not change the App or website dependency graph, operate Lintel
tasks, publish media, or configure hosting. The current composition is a
20-second, 1920×1080, 60 fps film with music/SFX and an optional English voice.
The tested production host is macOS arm64; the voice mux uses Remotion's bundled
macOS arm64 FFmpeg. Other production hosts have not been verified.

## Sources

- [`film.tsx`](../../assets/preview/film.tsx) owns the frame-derived camera,
  spatial character terrain, file streams, package, review sequence and bridge.
  The camera uses continuous cubic timing; no real-time animation loop runs
  during export. The bridge and actor use depth testing, a shared deck profile
  and a small whole-body gait. The cargo stays attached to the actor.
- [`prepare-preview-film.mjs`](../prepare-preview-film.mjs) projects the existing
  canonical mark/outlined wordmark and shared connected Clawd sprite. It does
  not redraw brand glyphs or magnify a small raster mascot. The existing night
  plate supplies only the distant upper horizon. Generated SVGs, source
  snapshot, browser bundle, poster, MP4s and manifest stay in the chosen output.
- [`prepare-audio.mjs`](prepare-audio.mjs) synthesizes an original 96 bpm score
  and finite SFX with Tone.js in disposable Chromium. Kokoro.js generates the
  four speech takes locally with the `af_heart` voice and q8 CPU model.
- [`master-audio.mjs`](master-audio.mjs) is the single mix owner. It retains
  original stems, rebalances music/SFX, ducks music by 60% around speech with
  220 ms attack / 360 ms release, and sets a PCM peak ceiling of −1.5 dBFS.
  The manifest reports PCM peak/RMS; it does not claim a universal LUFS target.
- [`player.html`](../../assets/preview/player.html) supplies an opt-in local
  player. Music-only and narration variants share the same encoded video;
  switching mixes retains the playback position and paused/playing state.

The native App demonstration is separate: eight caller-supplied real native
screenshots with synthetic data, edited into a 30-second 1600×1000 sequence.
This is labelled `REAL macOS APP · SYNTHETIC DATA · EDITED`, has no audio, and
does not constitute a continuous recording or authenticated-client acceptance.

## Install the isolated production runtime

From the repository root, with Node/npm and the existing browser-test Chromium:

```sh
mkdir -p candidate-packages/film-runtime
cp scripts/preview-film/package.json scripts/preview-film/package-lock.json candidate-packages/film-runtime/
npm ci --prefix candidate-packages/film-runtime
```

The lockfile pins Remotion 4.0.534, Three.js, React Three Fiber, Tone.js and
Kokoro.js. The runtime is ignored, separate from production dependencies.
Existing Playwright defaults to `extensions/browser/node_modules/playwright`;
the film renderer also accepts `PLAYWRIGHT_MODULE` for an existing installation.
Install browser-test dependencies through their own documented entry if absent.

## Prepare audio and render

Use absent output directories. First-time audio preparation downloads the public
Kokoro model and voice data into the ignored runtime's `model-cache`; inference
is local. It does not send script text or media to a speech service, use account
credentials, or make a paid API call. Music/SFX/voice stems and raw takes remain
available for later visual revisions without another generation.

```sh
mkdir candidate-packages/film-audio
node scripts/preview-film/prepare-audio.mjs candidate-packages/film-runtime candidate-packages/film-audio
node scripts/prepare-preview-film.mjs --out candidate-packages/film-export --audio-dir candidate-packages/film-audio
```

To revise only the mix from existing production stems:

```sh
node scripts/preview-film/master-audio.mjs candidate-packages/film-audio candidate-packages/film-audio-remix
```

For a still review, omit audio and pass `--stills`. `--runtime DIRECTORY` selects
another prepared runtime. `--native-captures DIRECTORY` additionally exports the
labelled native screenshot edit; inspect its exact expected names in the
generator and supply only synthetic, reviewed captures. Arbitrary supplied
screenshots are not authenticated by this script.

MP4 export writes `lintel-intro.mp4`, `lintel-intro-voice.mp4`, `index.html`,
`poster.png`, six review frames and `manifest.json`. The voice variant copies
the already encoded video and replaces only the audio track. The renderer uses
ANGLE with a fresh export browser after still review. A failed render leaves
its local evidence intact; use a new output directory after diagnosing it.

For opt-in local playback:

```sh
python3 -m http.server 4318 --bind 127.0.0.1 --directory candidate-packages/film-export
```

Open the loopback URL in a browser. The page has no third-party requests,
analytics or autoplay. This local player is not a website deployment.

## Review and rights

Check the actual MP4, including camera passage, text collisions, the character's
feet/cargo/foreground rail and the final orange foot. Inspect dimensions/frame
count/audio tracks, decode the complete media and measure the encoded audio
when mastering changes. A successful export does not establish aesthetic
acceptance, pronunciation quality, signing, release, or deployment. The
manifest's HEAD/dirty fields are a build declaration, not source authentication.

All MP4s, WAVs, model data, installed tools, browser bundles and QA captures stay
outside tracked source. Do not commit raw native data, binaries or model caches.
Remotion, Three.js, React Three Fiber, Tone.js and Kokoro retain their own terms:
review the [Remotion license](https://www.remotion.dev/license),
[Three.js source](https://github.com/mrdoob/three.js),
[React Three Fiber source](https://github.com/pmndrs/react-three-fiber),
[Tone.js source](https://github.com/Tonejs/Tone.js),
[Kokoro.js](https://github.com/hexgrad/kokoro/tree/main/kokoro.js) and
[model card](https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX).
Clawd belongs to Anthropic; Lintel is independent. No new public licence terms
for project-original material are introduced by this production toolchain.
