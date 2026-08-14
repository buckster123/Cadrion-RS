//! Clap surface.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "cadre",
    version,
    about = "Cadre — Rust-native CAD runtime for AI agents",
    long_about = None
)]
pub struct Cli {
    /// Machine-readable JSON on stdout (human text is a rendering of the same data).
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential human output.
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    /// Project root (default: cwd).
    #[arg(long, global = true, env = "CADRE_PROJECT")]
    pub project: Option<PathBuf>,

    /// Geometry kernel: mock (default) | occt | truck (seed) | truck-brep (H3-6 spike).
    #[arg(long, global = true, env = "CADRE_KERNEL", default_value = "mock")]
    pub kernel: KernelId,

    /// More logs on stderr.
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum KernelId {
    Mock,
    Occt,
    /// Experimental analytic CSG seed (NON-PARITY; never default).
    Truck,
    /// H3-6 upstream truck B-rep spike (NON-PARITY; feature `truck-brep`).
    TruckBrep,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Evaluate `.cad.star`, execute IR, write primary artifact (STEP when kernel supports it).
    Build(BuildArgs),
    /// Numeric interrogation (refs inventory / measure).
    Inspect(InspectArgs),
    /// Secondary format export from an existing STEP (or rebuild from source).
    Export(ExportArgs),
    /// Deterministic parity suite (`parts1-4`, …).
    Bench(BenchArgs),
    /// Agent-loop harness scorecard (`agent10`).
    Harness(HarnessArgs),
    /// Multi-view PNG packet (+ orbit GIF).
    Snapshot(SnapshotArgs),
    /// Local embedded viewer (deep links).
    View(ViewArgs),
    /// MCP server (stdio JSON-RPC; logs on stderr).
    Mcp(McpArgs),
    /// Skill-pack export for agents.
    Skills(SkillsArgs),
    /// Local HTTP API server.
    Serve(ServeArgs),
    /// Robot description gen/validate (URDF/SRDF/SDF).
    Robot(RobotArgs),
    /// Assembly spec validate (joints + components).
    Assembly(AssemblyArgs),
    /// Fabrication: DXF, DFM, slicer, gcode-check.
    Fab(FabArgs),
    /// Printer adapters (Bambu / Klipper dry-run / gated start).
    Printer(PrinterArgs),
    /// Clean-room build123d-style Python → Cadre `.cad.star` skeleton (best-effort).
    Migrate(MigrateArgs),
    /// Experimental secondary SDF sample (analytic box/cyl → raw/NRRD). Not modeling.
    Sdf(SdfArgs),
    /// Print versions / feature flags.
    Version,
}

#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Target `.cad.star` (explicit file only — no directory scans).
    pub target: PathBuf,

    /// Output path (default: same basename as target with .step or .ir.json).
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Parameter override `key=value` (repeatable).
    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,

    /// Bypass build cache.
    #[arg(long)]
    pub no_cache: bool,
}

#[derive(Debug, clap::Args)]
pub struct InspectArgs {
    #[command(subcommand)]
    pub cmd: InspectCmd,
}

#[derive(Debug, Subcommand)]
pub enum InspectCmd {
    /// List stable selector tokens (+ optional facts).
    Refs(RefsArgs),
    /// Measure distance / angle / diameter / thickness between refs.
    Measure(MeasureArgs),
    /// Mating / alignment check between two refs.
    Align(AlignArgs),
    /// Local frame (origin + axes) for a ref.
    Frame(FrameArgs),
    /// Diff two builds (volume/faces + selector remap hints).
    Diff(DiffArgs),
    /// PMI alpha: linear dimension facts → drawing packet JSON (not a drafting package).
    Dims(DimsArgs),
}

#[derive(Debug, clap::Args)]
pub struct RefsArgs {
    /// `.cad.star` source (topology derived from IR) or path ignored if --ir given.
    pub target: PathBuf,

    /// Attach aggregate facts summary.
    #[arg(long)]
    pub facts: bool,

    /// Parameter overrides when evaluating source.
    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct MeasureArgs {
    pub target: PathBuf,
    /// Selector A, e.g. `#o1.1.f1`
    pub a: String,
    /// Selector B (required for distance/angle/thickness).
    pub b: Option<String>,
    #[arg(long, value_enum, default_value_t = MeasureKindArg::Distance)]
    pub kind: MeasureKindArg,
    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct AlignArgs {
    pub target: PathBuf,
    pub a: String,
    pub b: String,
    #[arg(long, value_enum, default_value_t = AlignExpectArg::Distance)]
    pub expect: AlignExpectArg,
    /// Expected distance mm when --expect distance.
    #[arg(long)]
    pub distance: Option<f64>,
    #[arg(long, default_value_t = 0.1)]
    pub tol: f64,
    #[arg(long, default_value_t = 1.0)]
    pub tol_deg: f64,
    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AlignExpectArg {
    Coplanar,
    Coaxial,
    Distance,
}

#[derive(Debug, clap::Args)]
pub struct FrameArgs {
    pub target: PathBuf,
    /// Selector e.g. `#o1.1.f1`
    pub selector: String,
    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct DiffArgs {
    /// Older build source `.cad.star`
    pub old: PathBuf,
    /// Newer build source `.cad.star`
    pub new: PathBuf,
    #[arg(long = "set-old", value_name = "KEY=VAL")]
    pub set_old: Vec<String>,
    #[arg(long = "set-new", value_name = "KEY=VAL")]
    pub set_new: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct DimsArgs {
    /// `.cad.star` source (or IR json).
    pub target: PathBuf,
    /// Write drawing packet JSON here (default: `<stem>.drawing.json` next to target).
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Explicit dim: `A,B,kind` or `A,kind` for diameter. kind=distance|thickness|diameter|angle.
    /// Repeatable. If omitted, auto opposite-face linear dims.
    #[arg(long = "dim", value_name = "SPEC")]
    pub dim: Vec<String>,
    /// Optional JSON file of DimSpec array.
    #[arg(long = "specs")]
    pub specs: Option<PathBuf>,
    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MeasureKindArg {
    Distance,
    Angle,
    Diameter,
    Thickness,
}

#[derive(Debug, clap::Args)]
pub struct ExportArgs {
    /// Source `.cad.star` or existing `.step`.
    pub target: PathBuf,

    #[arg(long, value_enum)]
    pub format: ExportFormat,

    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    Step,
    Stl,
    Glb,
}

#[derive(Debug, clap::Args)]
pub struct BenchArgs {
    #[command(subcommand)]
    pub cmd: BenchCmd,
}

#[derive(Debug, Subcommand)]
pub enum BenchCmd {
    /// Run a deterministic suite (default: parts1-4).
    Run(BenchRunArgs),
}

#[derive(Debug, clap::Args)]
pub struct BenchRunArgs {
    /// Suite id: parts1-4 | parts5-10 | parts1-10 | parity10 | parts1-4-occt | fillet-occt
    #[arg(long, default_value = "parts1-4")]
    pub suite: String,

    /// Path to parity root (default: auto-detect `parity/` from cwd or crate layout).
    #[arg(long)]
    pub parity_root: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct HarnessArgs {
    #[command(subcommand)]
    pub cmd: HarnessCmd,
}

#[derive(Debug, Subcommand)]
pub enum HarnessCmd {
    /// Run agent-loop suite and print scorecard.
    Run(HarnessRunArgs),
}

#[derive(Debug, clap::Args)]
pub struct HarnessRunArgs {
    /// Suite id: agent10 (default)
    #[arg(long, default_value = "agent10")]
    pub suite: String,
    /// Path to harness/tasks (auto-detect by default).
    #[arg(long)]
    pub tasks_root: Option<PathBuf>,
    /// Live agent driver: shell command (`sh -c`). When set, runs live mode
    /// (prompt-only; agent must write part.cad.star). See harness/README.md.
    #[arg(long)]
    pub cmd: Option<String>,
    /// Per-loop agent timeout seconds (live mode; 0 = wait forever).
    #[arg(long, default_value_t = 300)]
    pub timeout: u64,
    /// Skip snapshot after live build (faster; fails tasks needing snapshot_ok).
    #[arg(long, default_value_t = false)]
    pub no_snapshot: bool,
}

#[derive(Debug, clap::Args)]
pub struct SnapshotArgs {
    /// Target `.cad.star` (preview mesh from IR) or existing `.snap` dir (re-render not yet).
    pub target: PathBuf,

    /// Comma-separated views: iso,front,top,right,back,left,bottom
    #[arg(long, default_value = "iso,front,top,right")]
    pub views: String,

    /// Output directory (default: `<stem>.snap` next to target).
    #[arg(long, short = 'o')]
    pub out: Option<PathBuf>,

    /// Image size (square).
    #[arg(long, default_value_t = 512)]
    pub size: u32,

    /// Skip orbit GIF.
    #[arg(long)]
    pub no_gif: bool,

    /// Orbit frame count.
    #[arg(long, default_value_t = 24)]
    pub gif_frames: u32,

    #[arg(long = "set", value_name = "KEY=VAL")]
    pub set: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct ViewArgs {
    /// Paths: `.snap` dirs, `.cad.star`, images, `.gcode` / `.gco`, `.robot.json` / `.urdf`.
    pub paths: Vec<PathBuf>,

    /// Bind port (default 7411).
    #[arg(long, default_value_t = 7411, env = "CADRE_VIEWER_PORT")]
    pub port: u16,

    /// Bind host.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Open once: prepare artifacts only, do not serve (CI-friendly).
    #[arg(long)]
    pub once: bool,
}

#[derive(Debug, clap::Args)]
pub struct McpArgs {
    // reserved for --http later
}

#[derive(Debug, clap::Args)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub cmd: SkillsCmd,
}

#[derive(Debug, Subcommand)]
pub enum SkillsCmd {
    /// Export bundled Cadre skill pack to a directory.
    Export(SkillsExportArgs),
}

#[derive(Debug, clap::Args)]
pub struct SkillsExportArgs {
    /// Output directory (default: dist/skills/cadre, or dist/skills with --all).
    #[arg(long, short = 'o')]
    pub out: Option<PathBuf>,

    /// Target agent ecosystem label (affects INSTALL.md).
    #[arg(long, default_value = "claude-code")]
    pub agent: String,

    /// Export packs for claude-code, codex, and hermes under `<out>/<agent>/cadre`.
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    #[command(subcommand)]
    pub cmd: ServeCmd,
}

#[derive(Debug, Subcommand)]
pub enum ServeCmd {
    /// Run local Axum HTTP API (`/v1/*`).
    Api(ServeApiArgs),
    /// Run streamable HTTP MCP (`POST /mcp`, `GET /mcp` SSE).
    Mcp(ServeMcpArgs),
}

#[derive(Debug, clap::Args)]
pub struct ServeApiArgs {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, default_value_t = 7410, env = "CADRE_API_PORT")]
    pub port: u16,
    /// Optional bearer token (also `CADRE_API_TOKEN`).
    #[arg(long, env = "CADRE_API_TOKEN")]
    pub token: Option<String>,
    /// Project root for relative paths (default: cwd / --project).
    #[arg(long)]
    pub project: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct ServeMcpArgs {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, default_value_t = 7420, env = "CADRE_MCP_PORT")]
    pub port: u16,
    /// Optional bearer token (also `CADRE_MCP_TOKEN`).
    #[arg(long, env = "CADRE_MCP_TOKEN")]
    pub token: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct RobotArgs {
    #[command(subcommand)]
    pub cmd: RobotCmd,
}

#[derive(Debug, Subcommand)]
pub enum RobotCmd {
    /// Validate a robot JSON spec (and emit) or existing .urdf/.srdf/.sdf.
    Validate(RobotValidateArgs),
    /// Generate URDF (+ optional SRDF/SDF) from robot JSON spec.
    Gen(RobotGenArgs),
}

#[derive(Debug, clap::Args)]
pub struct RobotValidateArgs {
    pub target: PathBuf,
    /// Optional paired file (e.g. SRDF against URDF).
    #[arg(long)]
    pub against: Option<String>,
    /// When target is robot JSON, also write artifacts here.
    #[arg(long)]
    pub emit_dir: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct RobotGenArgs {
    /// Path to robot JSON spec.
    pub spec: PathBuf,
    /// Output directory (default: next to spec).
    #[arg(long, short = 'o')]
    pub out: Option<PathBuf>,
    /// Also write SRDF.
    #[arg(long, default_value_t = true)]
    pub srdf: bool,
    /// Also write SDF.
    #[arg(long, default_value_t = true)]
    pub sdf: bool,
}

#[derive(Debug, clap::Args)]
pub struct AssemblyArgs {
    #[command(subcommand)]
    pub cmd: AssemblyCmd,
}

#[derive(Debug, Subcommand)]
pub enum AssemblyCmd {
    /// Validate assembly JSON (components + joints fail-closed).
    Validate(AssemblyValidateArgs),
    /// Emit kinematics sidecar JSON (mm→m, deg→rad). Not AP242.
    EmitKinematics(AssemblyEmitArgs),
    /// Emit minimal robot JSON (placeholder solids) for `cadre robot gen`.
    EmitRobot(AssemblyEmitArgs),
}

#[derive(Debug, clap::Args)]
pub struct AssemblyValidateArgs {
    /// Path to `.assy.json` assembly spec.
    pub target: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct AssemblyEmitArgs {
    /// Path to `.assy.json` assembly spec.
    pub target: PathBuf,
    /// Output path (default: `<stem>.kinematics.json` or `<stem>.robot.json`).
    #[arg(short = 'o', long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct FabArgs {
    #[command(subcommand)]
    pub cmd: FabCmd,
}

#[derive(Debug, Subcommand)]
pub enum FabCmd {
    /// Write a simple plate+holes DXF (mm).
    Dxf(FabDxfArgs),
    /// Project a planar face from a .cad.star model to DXF.
    DxfFace(FabDxfFaceArgs),
    /// DFM preflight against a vendor profile.
    Check(FabCheckArgs),
    /// List bundled DFM profile ids.
    Profiles,
    /// Discover local slicer CLIs.
    Slicers,
    /// Preview or gated-execute a slicer command.
    Slice(FabSliceArgs),
    /// Static G-code validation.
    GcodeCheck(FabGcodeCheckArgs),
}

#[derive(Debug, clap::Args)]
pub struct FabDxfArgs {
    #[arg(long, default_value_t = 100.0)]
    pub width: f64,
    #[arg(long, default_value_t = 50.0)]
    pub height: f64,
    /// Hole as cx,cy,diameter_mm (repeatable).
    #[arg(long = "hole")]
    pub hole: Vec<String>,
    #[arg(long, short = 'o')]
    pub out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct FabDxfFaceArgs {
    /// Part source (.cad.star or IR json).
    pub target: PathBuf,
    /// Face selector e.g. `#o1.1.f0` (optional if --normal given).
    #[arg(long)]
    pub face: Option<String>,
    /// Pick largest face with this normal `x,y,z` (default 0,0,1 if no --face).
    #[arg(long)]
    pub normal: Option<String>,
    #[arg(long, short = 'o')]
    pub out: Option<PathBuf>,
    /// Plane thickness tol mm for coplanar edges.
    #[arg(long, default_value_t = 0.5)]
    pub plane_tol: f64,
    #[arg(long)]
    pub set: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct FabCheckArgs {
    /// Bundled profile id (default: sendcutsend.laser). Also: pcb.outline
    #[arg(long, default_value = "sendcutsend.laser")]
    pub profile: String,
    /// Optional external profile JSON file.
    #[arg(long)]
    pub profile_file: Option<PathBuf>,
    /// FlatPart JSON file (overrides width/height/…).
    #[arg(long)]
    pub part_json: Option<PathBuf>,
    #[arg(long)]
    pub width: Option<f64>,
    #[arg(long)]
    pub height: Option<f64>,
    #[arg(long)]
    pub thickness: Option<f64>,
    #[arg(long)]
    pub material: Option<String>,
    #[arg(long = "hole-dia")]
    pub hole_dia: Vec<f64>,
    #[arg(long)]
    pub min_edge: Option<f64>,
    #[arg(long)]
    pub min_spacing: Option<f64>,
}

#[derive(Debug, clap::Args)]
pub struct FabSliceArgs {
    pub mesh: PathBuf,
    #[arg(long)]
    pub slicer: Option<String>,
    /// Explicit slicer binary path (bypasses discovery; tests / custom stubs).
    #[arg(long)]
    pub slicer_bin: Option<PathBuf>,
    #[arg(long, short = 'o')]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub profile: Option<String>,
    /// Run the host slicer (requires `--confirm SLICE`).
    #[arg(long, default_value_t = false)]
    pub execute: bool,
    /// Must be exactly `SLICE` when `--execute` is set.
    #[arg(long)]
    pub confirm: Option<String>,
    /// Optional allowlist entry (basename or absolute path); repeatable.
    #[arg(long = "allowlist")]
    pub allowlist: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct FabGcodeCheckArgs {
    pub gcode: PathBuf,
    #[arg(long)]
    pub bed_x: Option<f64>,
    #[arg(long)]
    pub bed_y: Option<f64>,
    #[arg(long)]
    pub bed_z: Option<f64>,
    #[arg(long)]
    pub max_hotend: Option<f64>,
    #[arg(long)]
    pub max_bed: Option<f64>,
}

#[derive(Debug, clap::Args)]
pub struct PrinterArgs {
    #[command(subcommand)]
    pub cmd: PrinterCmd,
}

#[derive(Debug, Subcommand)]
pub enum PrinterCmd {
    /// Local metadata status (no MQTT poll yet).
    Status(PrinterStatusArgs),
    /// Dry-run: gcode-check + sha256 (no network).
    DryRun(PrinterDryRunArgs),
    /// Gated start. Network only with --live after allowlist+hash+confirm=START.
    Start(PrinterStartArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum PrinterBackend {
    #[default]
    Bambu,
    Klipper,
    Moonraker,
    Octoprint,
}

#[derive(Debug, clap::Args)]
pub struct PrinterStatusArgs {
    /// Backend: bambu (default) or klipper/moonraker.
    #[arg(long, value_enum, default_value_t = PrinterBackend::Bambu)]
    pub backend: PrinterBackend,
    #[arg(long, default_value = "bambu:x1c-01")]
    pub id: String,
    #[arg(long, default_value = "192.168.1.50")]
    pub host: String,
    #[arg(long, default_value = "X1C")]
    pub model: String,
    #[arg(long, env = "CADRE_BAMBU_SERIAL")]
    pub serial: Option<String>,
    /// Moonraker base URL (default: http://HOST:7125). Env: CADRE_MOONRAKER_URL.
    #[arg(long, env = "CADRE_MOONRAKER_URL")]
    pub moonraker_url: Option<String>,
    /// Moonraker API key. Env: CADRE_MOONRAKER_API_KEY.
    #[arg(long, env = "CADRE_MOONRAKER_API_KEY")]
    pub api_key: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct PrinterDryRunArgs {
    pub gcode: PathBuf,
    #[arg(long, value_enum, default_value_t = PrinterBackend::Bambu)]
    pub backend: PrinterBackend,
    #[arg(long, default_value = "bambu:x1c-01")]
    pub id: String,
    #[arg(long, default_value = "192.168.1.50")]
    pub host: String,
    #[arg(long, default_value = "X1C")]
    pub model: String,
    #[arg(long, env = "CADRE_BAMBU_SERIAL")]
    pub serial: Option<String>,
    #[arg(long, env = "CADRE_BAMBU_ACCESS_CODE")]
    pub access_code: Option<String>,
    #[arg(long, env = "CADRE_MOONRAKER_URL")]
    pub moonraker_url: Option<String>,
    #[arg(long, env = "CADRE_MOONRAKER_API_KEY")]
    pub api_key: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct PrinterStartArgs {
    pub gcode: PathBuf,
    /// SHA-256 of the gcode file (must match file on disk).
    #[arg(long)]
    pub sha256: String,
    /// Must be exactly START (human consent gate).
    #[arg(long)]
    pub confirm: Option<String>,
    #[arg(long, value_enum, default_value_t = PrinterBackend::Bambu)]
    pub backend: PrinterBackend,
    #[arg(long, default_value = "bambu:x1c-01")]
    pub id: String,
    #[arg(long, default_value = "192.168.1.50")]
    pub host: String,
    #[arg(long, default_value = "X1C")]
    pub model: String,
    /// Comma-separated allow-list of printer ids.
    #[arg(long)]
    pub allowlist: Option<String>,
    /// Opt-in to real network after gates pass (Bambu FTPS+MQTT or Moonraker HTTP).
    #[arg(long, default_value_t = false)]
    pub live: bool,
    /// Printer serial (MQTT topic, Bambu). Env: CADRE_BAMBU_SERIAL.
    #[arg(long, env = "CADRE_BAMBU_SERIAL")]
    pub serial: Option<String>,
    /// LAN access code (Bambu). Env: CADRE_BAMBU_ACCESS_CODE.
    #[arg(long, env = "CADRE_BAMBU_ACCESS_CODE")]
    pub access_code: Option<String>,
    /// Moonraker base URL. Env: CADRE_MOONRAKER_URL.
    #[arg(long, env = "CADRE_MOONRAKER_URL")]
    pub moonraker_url: Option<String>,
    /// Moonraker API key. Env: CADRE_MOONRAKER_API_KEY.
    #[arg(long, env = "CADRE_MOONRAKER_API_KEY")]
    pub api_key: Option<String>,
    /// Remote filename on printer (default: local basename).
    #[arg(long)]
    pub remote_name: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct MigrateArgs {
    /// build123d-style Python source (single file).
    pub source: PathBuf,
    /// Output `.cad.star` (default: `<stem>.cad.star` beside source).
    #[arg(short = 'o', long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct SdfArgs {
    #[command(subcommand)]
    pub cmd: SdfCmd,
}

#[derive(Debug, clap::Subcommand)]
pub enum SdfCmd {
    /// Sample analytic box/cylinder SDF → raw f32 + NRRD (secondary medium).
    Sample(SdfSampleArgs),
}

#[derive(Debug, clap::Args)]
pub struct SdfSampleArgs {
    /// Primitive: `box` or `cylinder`.
    #[arg(long, value_enum)]
    pub prim: SdfPrimArg,
    /// Box dx or cylinder radius (mm).
    #[arg(long)]
    pub a: f64,
    /// Box dy or cylinder height (mm).
    #[arg(long)]
    pub b: f64,
    /// Box dz (required for box).
    #[arg(long)]
    pub c: Option<f64>,
    /// Samples along longest axis (default 32).
    #[arg(long, default_value_t = 32)]
    pub res: usize,
    /// Padding around bounds (mm).
    #[arg(long, default_value_t = 2.0)]
    pub pad: f64,
    /// Output directory (default: `./sdf_out`).
    #[arg(short = 'o', long)]
    pub out: Option<PathBuf>,
    /// File stem (default: prim name).
    #[arg(long)]
    pub stem: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SdfPrimArg {
    Box,
    Cylinder,
}
