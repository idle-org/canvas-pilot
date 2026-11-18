# Presentation switcher adapter candidates

Initial research: **2026-11-15**; ATEM/vMix follow-up: **2026-11-18**. This is a documentation and feasibility survey for
Canvas Pilot's [stretch goals](plan.md#stretch-goals-distant-future), not a support
announcement or purchasing guide. Analog Way comes first; other manufacturers
follow in documentation tiers. Models sharing a control family are grouped.

Scope: major live-event presentation switchers and multi-window processors,
including relevant legacy equipment and adjacent production switchers. This is
not an exhaustive inventory of every regional product, discontinued SKU or OEM
variant. A manufacturer name or shared protocol never implies compatibility
across its range. Unknowns below mean **not established by this survey**, not
that integration is impossible.

## How the ranking works

Documentation rank measures availability, not product quality or market share:

| Tier | Evidence available | Meaning for planning |
| --- | --- | --- |
| A | Public, detailed programming reference or API schema | We can investigate an adapter without first requesting a private protocol. |
| B | Official protocol/SDK download exists, but package, version coverage or geometry details need further inspection | Promising; obtain and audit the exact reference before estimating work. |
| C | Product manuals, integration packages or external-control claims found, but no complete public geometry/readback contract established | Request the missing interface documentation before committing to support. |
| D | No verified model-specific programming reference located in this survey | Research only; do not plan implementation yet. |

Rows are ordered by tier within each section; equal tiers are ties. The
**feasibility assessments are engineering judgments**, not vendor guarantees.
A useful Canvas Pilot adapter must discover destinations and dimensions, read
layer geometry/source framing, write selected geometry, and verify the result.
Preset recall, TAKE, routing or tally alone do not meet that requirement.

## Analog Way first

| Tier | Family / representative models | Documentation evidence | Canvas Pilot feasibility and next step |
| --- | --- | --- | --- |
| A | **LivePremier / Aquilon**: C/C+, Cmax and RS variants | Public [AWJ 6.2 programmer's guide](https://dwn01.analogway.com/Site+Internet/Series/Aquilon/Common/AWJ+Protocol/AW_Aquilon_AWJ_ProgrammersGuide_V6.2.pdf). | **First-class supported for standalone AQL Cmax 6.2.x only; emulator-validated on 6.2.79.** Extend model/firmware profiles after fixtures and acceptance checks; linked systems need explicit unit identity and topology handling. See [current acceptance limits](aquilon-compatibility.md). |
| A | **Midra 4K**: Eikos 4K, Pulse 4K, QuickVu 4K, QuickMatrix 4K | AWJ and REST guides in the [official downloads](https://www.analogway.com/downloads); [vendor FAQ](https://www.analogway.com/midra-4k-faqs) documents control options and simulator. | **Existing adapter.** Broaden tested coverage deliberately; current Eikos hardware evidence does not certify every model/mode. See [hardware checks](device-check.md). |
| B | **Alta 4K**: Zenith 100 / 200 | AWJ v1.2 and REST guides plus simulator listed in [official downloads](https://www.analogway.com/downloads); [family overview](https://www.analogway.com/alta-4k-faqs). | **Best next new AW family to investigate.** Likely reuse transport and normalized geometry, but audit native paths, layers, banks and apply behavior. No Alta protocol package or device was exercised here. |
| B | **LiveCore**: Ascender 16/32/48 and LE/4K variants; NeXtage 08/16; SmartMatriX Ultra | [NeXtage support page](https://www.analogway.com/products/nextage-08-livecore) lists the LiveCore programmer's guide and simulator; [Ascender page](https://www.analogway.com/products/ascender-16) lists the family documentation. | **Plausible separate legacy adapter.** Inspect the programmer package and firmware-specific command tables; do not assume AWJ compatibility. Validate linked processors and output expanders separately. |
| B | **Legacy Midra (non-4K)**: Eikos², Pulse², QuickVu, QuickMatriX; Saphyr / SmartMatriX² variants need individual confirmation | Legacy Midra programmer's guide and AW Gateway listed in [official downloads](https://www.analogway.com/downloads). | **Plausible separate adapter.** Confirm command coverage, readback and connection-sharing limits. The existing Midra 4K adapter is not evidence of support for this generation. |
| D | **Earlier generations**: Pulse PLS300, legacy Di-VentiX, Eikos and other pre-Midra models | No exact current manufacturer-hosted protocol package verified during this survey. Start with the [vendor archive](https://www.analogway.com/downloads). | **Unknown per model.** Obtain the exact manual/command set and hardware before estimating; avoid one generic “old Analog Way” adapter. |

**Adjacent AW products:** [VIO 4K](https://www.analogway.com/products/vio-4k)
is a conversion/scaling candidate rather than the first choice for a multi-layer
editor adapter. Picturall media-server integration and control surfaces such as
Vertige/LiveTakes belong to separate workflow investigations, not extra switcher
profiles. Expanders should be represented through their host topology.

## Other presentation and multi-window processors

| Tier | Family / representative models | Documentation evidence | Canvas Pilot feasibility and next step |
| --- | --- | --- | --- |
| A | **Barco Event Master**: E2 / E2 Gen 2 / E2 Jr, S3 variants, **ENCORE3** | Public [remote-control reference R5919184](https://www.barco.com/en/support/docs/R5919184), with online edition and version history; [device guide](https://www.barco.com/en/support/docs/R5905948) documents JSON-RPC. | **High-value candidate; conditional geometry support.** Audit the exact toolset's layer discovery/read/write methods first. The presence of JSON-RPC alone does not prove complete geometry round trips. Model, toolset and linked-frame differences need profiles. |
| A | **Barco PDS-4K / PDS-4K SDI** | Listed in the same [remote-control reference](https://www.barco.com/en/support/docs/R5919184). | **Promising smaller target.** Check its implemented command subset and layer limits independently of E2; do not inherit capabilities just because it shares a reference. |
| A | **tvONE CORIOmaster / mini / micro** | Public [API portal](https://api.tvone.com/products/c3-series/c3-5xx/index.html): REST, JSON schemas, WebSocket events and CLI. | **Strong candidate.** Audit canvas/window coordinates, crop, rotation and staging behavior. A video-wall canvas may differ from one physical output; preserve that distinction in the adapter. |
| A | **tvONE CORIOmaster2 and CALICO C7 PRO** | Separate product APIs in the [vendor API directory](https://api.tvone.com/). | **Strong candidates, separate profiles.** Compare their schemas with CORIOmaster before deciding what transport/conversion code is genuinely reusable. |
| A | **Extron MGP 641 / MGP 641 xi / xi 5K**; older MGP generations separately | Public [MGP 641 SIS command tables](https://media.extron.com/public/download/files/userman/68-2396-01_C_MGP_641_UG.pdf); [xi 5K setup guide](https://media.extron.com/public/download/files/userman/68-2396-52_A_MGP_641_xi_5K_Series_SUG.pdf) points to its own full manual. | **Strong bounded multi-window candidate.** Confirm window size/position and source framing queries per generation. Model native live-update behavior explicitly; do not invent an AW-style Preview bank. Newer profiles may require SSH transport. |
| A | **NovaStar H series**: H2/H5/H9/H15/H20 family | Public [H OpenAPI](https://openapi.novastar.tech/en/h/), including [layer details](https://openapi.novastar.tech/en/h/api-365455770) and [layer edits](https://openapi.novastar.tech/en/h/api-365455773). | **Strong candidate.** Audit authentication/project keys, coordinate units, layer limits and preset versus live editing. This is considerably stronger evidence than an “open API” marketing claim. Exact firmware coverage still needs validation. |
| A | **Christie Spyder X80** | Public [serial-command reference](https://www.christiedigital.com/globalassets/resources/public/020-102207-08-christie-lit-man-ref-spyder-commands.pdf), including layer position, size and combined `LSP` commands. | **Plausible, with a readback gate.** Geometry writes are documented; prove complete current layer/crop/source and PixelSpace discovery before promising bidirectional support. UDP response handling and PixelSpace mapping need care. |
| A | **Christie Spyder X20** | Public [user manual with external-control protocol](https://www.christiedigital.com/globalassets/020-000916-01-lit-man--usr-spyder-x20.pdf). | **Plausible legacy candidate.** Audit independently of X80; establish state queries and framing conversion. Public write commands are not sufficient acceptance evidence. |
| A | **Kramer VP-771 / VP-772** | Public [VP-771 manual](https://cdn.kramerav.com/web/downloads/manuals/vp-771_rev_8.pdf) and [VP-772 manual](https://cdn.kramerav.com/web/downloads/manuals/vp-772_rev_4.pdf) cover window controls and remote command protocols. | **Possible limited adapter.** Confirm window geometry queries and supported modes; do not generalize to Kramer matrix switchers without composition layers. |
| B | **tvONE CORIOview** | Product-specific APIs linked from the [vendor API directory](https://api.tvone.com/). | **Conditional fit.** Establish whether the desired arbitrary geometry is editable or constrained to supported layouts; a layout selector alone is not equivalent to Canvas Pilot's editor. |
| C | **PIXELHUE P10 / P20 / P20-DS / Q8 / P80** | [Official downloads](https://www.pixelhue.com/index.php/Content/index/catid/7/id/0.html) and [official archive](https://community.pixelhue.com/t/pixelhue-software-firmware-archive-version-history-downloads/26) include software and integration packages. | **Relevant but documentation-blocked.** Obtain a versioned geometry/state protocol. Existing Companion/Crestron/Q-SYS packages do not by themselves establish arbitrary layer read/write support. |
| C | **RGBlink X series**, starting with X7; X3/X-series variants separately | [X7 product/download page](https://www.rgblink.com/x7) provides manuals. | **Conditional.** Request the model-specific external-control/SDK reference and verify discovery, geometry and readback. No full public contract was established here. |
| C | **Magnimage MIG series**: H8/H9 and related event processors | [Manufacturer download centre](https://www.magnimage.com/article/download) lists control software and product documentation. | **Conditional.** Obtain processor-specific protocol documentation before estimating. Controller software availability is not an API specification. |
| C | **Legacy Barco/Folsom**: ScreenPRO-II, original Encore, PresentationPRO and older PDS | [ScreenPRO-II support archive](https://www.barco.com/en/support/screenpro-ii-series/documentation) remains available. | **Research per generation.** Locate exact geometry/readback commands and hardware. Original Encore must not be confused with ENCORE3/Event Master. |

## Adjacent production switchers and LED controllers

These are worth recording, but their composition models may only support a
subset of Canvas Pilot. Documentation grades still refer to control access,
not proof of arbitrary multi-layer editing.

| Tier | Family | Documentation evidence | Feasibility |
| --- | --- | --- | --- |
| A | **Blackmagic Design ATEM SuperSource**, compatible models only | Detailed official [ATEM SDK manual](https://documents.blackmagicdesign.com/DeveloperManuals/ATEMSDKManual.pdf?_v=1768464010000): box geometry/crop getters, setters and callbacks. | **Promising bounded bidirectional adapter.** Windows/macOS SDK; Linux integration and SuperSource-capable test target remain unresolved. See the focused assessment below. |
| A | **vMix input Layers** | Official [HTTP API](https://www.vmix.com/help29/DeveloperAPI.html) and [layer function reference](https://www.vmix.com/help29/ShortcutFunctionReference.html). | **Accessible send candidate; full readback unproven.** Pixel rectangles, pan/zoom and crop writes are documented. Verify complete layer geometry/crop reads before promising Load or verified Send. See below. |
| A | **Roland V-600UHD** | Public [LAN/RS-232 reference](https://static.roland.com/assets/media/pdf/V-600UHD_reference_eng05_W.pdf). | **Potential PinP/ROI subset.** Audit geometry read/write parameters and convert native scaling units explicitly. |
| A | **Ross Carbonite / Graphite / Acuity** | Public [RossTalk reference](https://documentation.rossvideo.com/files/Manuals/Protocol%20Documents/RossTalk-Commands%284802DR-403%29.pdf). | **Not yet a full geometry candidate.** Prove continuous DVE geometry and state feedback through an appropriate interface; macro/bus control alone is insufficient. |
| A | **NovaStar COEX / MX / KU / CX controllers** | Public [COEX API manual](https://oss.novastar.tech/uploads/2023/02/COEX-Series-Interface-API-User-Manual.pdf). | **Low priority for this editor.** The inspected reference establishes controller/source/preset functions, not a general layer-composition contract. Do not reuse H-series API assumptions. |

| B | **Roland V-160HD / V-1200HD / V-800HD and other V-series** | [Official protocol index](https://support.roland.com/hc/en-us/articles/360025923832-Professional-A-V-LAN-and-RS-232-Control-Tips-and-Documentation); [V-160HD details](https://support.roland.com/hc/en-us/articles/7816887053083-V-160HD-LAN-and-RS-232-Control-Tips). | **Possible model-specific subsets.** Transport, command encoding and parameter coverage differ; V-160HD uses SysEx wrappers and some commands require additional documentation. |
| B | **Panasonic AV-UHS500 / AV-HS6000 / AV-HS410** | [Official protocol document index](https://eww.pass.panasonic.co.jp/pro-av/support/content/guide/EN/top.html). | **Conditional, lower priority.** Audit exact external-control/plugin references for DVE state and geometry; bus, macro and tally access is not enough. |
| B | **Datavideo SE-3200**; other SE models separately | [Official product downloads](https://datavideo.com/in/product/SE-3200) list its RS-232 control protocol. | **Possible PinP subset.** Inspect parameter readback and geometry coverage before considering an adapter. Do not assume DVIP and serial command parity. |

Plain HDMI/SDI matrices, distribution devices and LED sending-only controllers
are not initial geometry-adapter targets. vMix is included as a specific software
composition candidate; other software mixers and media servers remain outside
this survey.

## ATEM SuperSource and vMix: focused feasibility assessment

These are research targets, not announced support or implementation commitments.
The assessments below come from documentation, not a tested SDK or running device.

### Blackmagic ATEM SuperSource

The official SDK documents position, size, crop edges, enabled state and change
callbacks for SuperSource boxes. That makes both Load and Send plausible. Start
with SuperSource on an explicitly supported model; DVE/keyer support would be a
separate capability investigation, not implied support for every ATEM.

The SDK documents Windows and macOS. Linux would require an assessed community
protocol implementation or a bridge, with additional packaging and maintenance
costs. An SDK reference is not a published cross-platform wire protocol.
Translate box scaling/cropping into our geometry deliberately; independent crop
and mask behavior must be proven rather than assumed equivalent to Midra.

Testing is the current gate. A community [ATEM Mini network emulator](https://github.com/tin921/atem-mini-emulator)
exists, but its documentation does not establish SuperSource coverage. Do not
count it as satisfying our API-emulator prerequisite until the required commands,
state reads and callbacks are verified. A visual preset simulator alone is not
an API test target; physical output still needs separate acceptance.

**Feedback:** worthwhile, moderate integration effort. Documentation is a strength;
Linux deployment and a suitable SuperSource test target are the larger unknowns.

### vMix input Layers

The function reference documents layer rectangles in pixels, zoom, pan and crop.
A proposed destination is **connection → composition input → layer slot**;
use the API's input GUID rather than an input number or editable name. This is
an adapter-model proposal, not a persistence or GUI change.

The HTTP API exposes XML state, but the inspected reference does not establish
complete current geometry/crop readback for every layer. Confirm that first.
HTTP success is not geometry verification; never reconstruct purported current
state solely from the last commands we sent. Crop/mask equivalence, native units,
layer limits and live-update behavior also need actual testing.

vMix itself can be the API test target on Windows, avoiding a separate hardware
emulator. Its [60-day trial includes Pro features](https://www.vmix.com/knowledgebase/article.aspx/59/which-edition-is-the-trial);
ongoing development needs continued access to an installation. The network API
could be called by Canvas Pilot on other platforms; vMix remains the Windows host.
This real software endpoint would serve the purpose of our emulator prerequisite.

**Feedback:** likely the easiest initial send experiment of these two candidates.
Full bidirectional support remains conditional on readback. Investigate vMix XML
coverage first, then ATEM SuperSource emulator coverage and Linux integration.
Any eventual send-only support must be explicitly scoped and approved, not
silently presented as satisfying our existing adapter acceptance requirements.

## Suggested investigation order

This is an implementation-priority judgment, distinct from the documentation
ranking above:

1. **Alta 4K**, alongside broader tested Midra 4K/Aquilon profiles: closest to the
   current architecture and operator workflow.
2. **Barco Event Master**, starting with one available E2/S3/PDS-4K target:
   investigate the layer read/write contract before promising support.
3. **tvONE CORIOmaster or Extron MGP**, whichever hardware is available: good
   public references and a useful test of the intermediate representation.
4. **NovaStar H**, then **Christie Spyder X80** once complete readback is proven.
5. **vMix / ATEM SuperSource** for a composition-focused expansion: resolve vMix
   readback and ATEM testing/platform gates first. **LiveCore / legacy Midra**
   when there is an equipment need; **Roland PinP** for a constrained workflow.
6. **PIXELHUE, RGBlink and Magnimage** after obtaining the missing protocol
   evidence. Documentation discovery can move them up; their current tier is
   not a statement about hardware capability.

## Keep future adapters small and first class

Use the existing [adapter contract](../interfaces/device-adapters.md): normalize
native screens/canvases, stable layer IDs, dimensions, source framing and
supported geometry into the shared representation. Keep native units, paths,
bank semantics and transports in each family implementation. Share proven
mechanisms, not assumed similarities between vendors.

Before accepting a new adapter, establish:

- A specific model/firmware profile and legal access to its programming interface.
- Fresh bounded state reads, stable destination identity, crop/geometry semantics
  and explicit behavior for live-only devices or absent Preview/Program banks.
- Selective writes, stale-state checks and actual geometry readback; unsupported
  transforms must remain explicit rather than silently approximated.
- Sanitized fixtures, transport/conversion regression checks and controlled
  emulator or hardware acceptance, followed by physical-output inspection.

Input thumbnails are optional capabilities, not a prerequisite for basic
geometry exchange. Do not build a generic plugin framework, general show-control
engine or SDK abstraction merely to accommodate this list. Extend persistence
and document migration only when a concrete adapter needs it. Multi-screen
project editing remains its own stretch goal.

No new device, simulator or SDK was tested for this survey. Only the existing
Midra/Aquilon acceptance records represent completed integration work.
