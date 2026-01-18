# Pedaler

A real-time circuit simulator for guitar pedals written in Rust.

Pedaler simulates analog guitar effect circuits at audio sample rate, allowing you to process audio through virtual pedal circuits defined in a simple text-based DSL.

## Features

- **Real-time simulation** using Modified Nodal Analysis (MNA)
- **Linear components**: Resistors, Capacitors, Inductors
- **Nonlinear components**: Diodes, BJTs, Op-Amps
- **Control elements**: Potentiometers, Switches
- **Sources**: DC/AC Voltage sources, Current sources
- **Digital effects**: Delay lines, FDN Reverb (placeable anywhere in circuit)
- **LFO modulation**: Time-varying components for phaser/flanger effects
- **Simple DSL** for circuit description (`.ped` files)
- **CLI tool** for processing audio via stdin/stdout
- **WASM target** for web audio applications (coming soon)

## Building

### Prerequisites

- Rust 1.70+ (edition 2021)
- Cargo

### Build from source

```bash
# Clone the repository
git clone https://github.com/pedaler/pedaler_core.git
cd pedaler_core

# Build release binary
cargo build --release

# The binary will be at target/release/pedaler
```

### Install locally

```bash
cargo install --path .
```

## CLI Usage

The `pedaler` CLI reads raw PCM audio from stdin, processes it through the circuit simulation, and writes the result to stdout.

### Basic Usage

```bash
pedaler <CIRCUIT_FILE> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `CIRCUIT_FILE` | Path to the circuit description file (`.ped`) |

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --sample-rate <HZ>` | Sample rate in Hz | 48000 |
| `-i, --max-iterations <N>` | Maximum Newton-Raphson iterations for nonlinear components | 50 || `-t, --tolerance <V>` | Convergence tolerance in volts (higher = faster, less precise) | 1e-4 || `-h, --help` | Print help information | |
| `-V, --version` | Print version information | |

### Audio Format

- **Format**: 32-bit floating point, little-endian (`f32le`)
- **Channels**: Mono (1 channel)
- **Sample Rate**: 48000 Hz (configurable via `--sample-rate`)

### Processing Audio with FFmpeg

The typical workflow uses FFmpeg to convert audio to/from the raw PCM format:

```bash
# Basic usage: process input.wav through a circuit
ffmpeg -i input.wav -f f32le -ac 1 -ar 48000 -hide_banner -loglevel error - \
  | pedaler my_circuit.ped \
  | ffmpeg -f f32le -ac 1 -ar 48000 -i - -hide_banner -loglevel error output.wav
```

#### Step-by-step breakdown:

1. **Input conversion** (first ffmpeg):
   - `-i input.wav` - Input audio file
   - `-f f32le` - Output format: 32-bit float, little-endian
   - `-ac 1` - Convert to mono
   - `-ar 48000` - Resample to 48kHz
   - `-` - Write to stdout

2. **Circuit simulation** (pedaler):
   - Reads PCM from stdin
   - Processes through circuit model
   - Writes PCM to stdout

3. **Output conversion** (second ffmpeg):
   - `-f f32le -ac 1 -ar 48000 -i -` - Read raw PCM from stdin
   - `output.wav` - Write to output file

### Examples

Process through the RC low-pass filter:
```bash
ffmpeg -i guitar.wav -f f32le -ac 1 -ar 48000 - \
  | pedaler examples/rc_lowpass.ped \
  | ffmpeg -f f32le -ac 1 -ar 48000 -i - filtered.wav
```

Process through the diode clipper (distortion):
```bash
ffmpeg -i guitar.wav -f f32le -ac 1 -ar 48000 - \
  | pedaler examples/diode_clipper.ped \
  | ffmpeg -f f32le -ac 1 -ar 48000 -i - distorted.wav
```

Use a different sample rate (96kHz):
```bash
ffmpeg -i input.wav -f f32le -ac 1 -ar 96000 - \
  | pedaler examples/rc_lowpass.ped --sample-rate 96000 \
  | ffmpeg -f f32le -ac 1 -ar 96000 -i - output.wav
```

## Circuit DSL Reference

Circuits are described in `.ped` files using a SPICE-inspired syntax.

> [!NOTE]
> **📖 Full Reference:** See [docs/dsl_reference.md](docs/dsl_reference.md) for complete syntax documentation, grammar specification, and more examples.

### Basic Syntax

- One component per line
- Comments start with `#` or `;`
- Node `0` or `GND` is ground
- Engineering notation supported: `p`, `n`, `u`, `m`, `k`, `M`, `G`

### Components

| Prefix | Component | Syntax | Example |
|--------|-----------|--------|---------|
| `R` | Resistor | `R<name> <n+> <n-> <value>` | `R1 in out 10k` |
| `C` | Capacitor | `C<name> <n+> <n-> <value>` | `C1 in 0 100n` |
| `L` | Inductor | `L<name> <n+> <n-> <value>` | `L1 in out 10m` |
| `D` | Diode | `D<name> <anode> <cathode> <model>` | `D1 in out DCLIP` |
| `Q` | BJT | `Q<name> <C> <B> <E> <model>` | `Q1 vcc base 0 NPN` |
| `V` | Voltage Source | `V<name> <n+> <n-> <type> [value]` | `V1 in 0 AC` |
| `I` | Current Source | `I<name> <n+> <n-> <value>` | `I1 vcc 0 1m` |
| `OP` | Op-Amp | `OP<name> <n+> <n-> <out> <model>` | `OP1 np nm out IDEAL` |
| `POT` | Potentiometer | `POT<name> <n1> <wiper> <n2> <value> <pos>` | `POT1 in w out 100k 0.5` |
| `SW` | Switch | `SW<name> <n1> <n2> <state>` | `SW1 in out CLOSED` |
| `DELAY` | Delay Line | `DELAY <name> <in> <out> <time> [mix=X] [feedback=Y]` | `DELAY d1 in out 300m mix=0.5 feedback=0.4` |
| `REVERB` | FDN Reverb | `REVERB <name> <in> <out> [params]` | `REVERB r1 in out decay=0.6 size=0.5` |
| `LFO` | Low Frequency Oscillator | `LFO <name> <rate> <shape>` | `LFO lfo1 0.5 sine` |

### Directives

| Directive | Purpose | Example |
|-----------|---------|---------|
| `.input <node>` | Mark audio input node | `.input in` |
| `.output <node>` | Mark audio output node | `.output out` |
| `.model <name> <type> (<params>)` | Define component model | `.model DCLIP D (vf=0.3 is=1e-9 n=1.8)` |

### Model Parameters

**Diode (D)**:
- `vf` - Forward voltage (V)
- `is` - Saturation current (A)
- `n` - Ideality factor

**BJT (NPN/PNP)**:
- `bf` - Forward beta
- `is` - Saturation current (A)

**Op-Amp (OP)**:
- `gain` - Open-loop gain
- `rin` - Input resistance (Ω)
- `rout` - Output resistance (Ω)

> [!NOTE]
> **📖 Detailed Models:** See [docs/components.md](docs/components.md) for component physics, mathematical equations, and MNA stamping details.

### Digital Effect Parameters

**Delay (DELAY)**:
- `time` - Delay time (e.g., `300m` = 300ms, `0.5` = 500ms)
- `mix` - Dry/wet mix, 0.0-1.0 (default: 0.5)
- `feedback` - Feedback amount, 0.0-1.0 (default: 0.0)

**Reverb (REVERB)**:
- `decay` - Reverb decay, 0.0-1.0 (default: 0.5)
- `size` - Room size, 0.0-1.0 (default: 0.5)
- `damping` - High-frequency damping, 0.0-1.0 (default: 0.3)
- `mix` - Dry/wet mix, 0.0-1.0 (default: 0.5)
- `predelay` - Initial delay before reverb (default: 0)

**LFO (LFO)**:
- `rate` - Oscillation frequency in Hz
- `shape` - Waveform: `sine`, `triangle`, `sawtooth`, `square`

### Modulated Components

Resistors can be modulated by an LFO for phaser/flanger effects:

```text
LFO LFO1 0.5 sine
R_MOD n1 n2 10k LFO1 depth=0.8 range=2.0
```

- `depth` - Modulation depth, 0.0-1.0
- `range` - Modulation range multiplier
- Formula: `R_effective = R_base * (1 + depth * range * lfo_value)`

### Example Circuit

```text
# Diode Hard Clipper
V_IN    in      0       AC
R_IN    in      n1      10k
D1      n1      0       DCLIP
D2      0       n1      DCLIP
R_OUT   n1      out     1k

.model DCLIP D (vf=0.3 is=1e-9 n=1.8)

.input  in
.output out
```

## Example Circuits

The `examples/circuits/` directory contains ready-to-use circuit files:

### Basic Circuits
| File | Description |
|------|-------------|
| `rc_lowpass.ped` | First-order RC low-pass filter (fc ≈ 1.59 kHz) |
| `treble_boost.ped` | Simple treble boost circuit |

### Distortion/Overdrive
| File | Description |
|------|-------------|
| `diode_clipper.ped` | Symmetrical diode hard clipper |
| `opamp_overdrive.ped` | Tube Screamer-style op-amp overdrive |
| `distortion.ped` | Heavy distortion circuit |
| `fuzz.ped` | Fuzz pedal circuit |
| `lofi.ped` | Lo-fi distortion effect |

### Time-Based Effects
| File | Description |
|------|-------------|
| `delay.ped` | Simple delay (300ms echo) |
| `slapback.ped` | Short slapback delay |
| `reverb.ped` | Basic FDN reverb |
| `hall_reverb.ped` | Large hall reverb |
| `distortion_reverb.ped` | Distortion into reverb chain |

### Modulation Effects
| File | Description |
|------|-------------|
| `phaser.ped` | 4-stage all-pass phaser with LFO |
| `flanger.ped` | 6-stage flanger with feedback |

### Effect Chains
| File | Description |
|------|-------------|
| `delay_hall_phaser.ped` | Delay → Hall Reverb → Phaser chain |
| `incircuit_delay.ped` | Demonstrates in-circuit delay placement |

## WASM / Web Usage

Pedaler Core compiles to WebAssembly for browser-based real-time audio processing using the Web Audio API.

### Quick Start

```bash
# Build WASM package
wasm-pack build --target web --features wasm --no-default-features
```

This produces a `pkg/` directory containing:
- `pedaler_core_bg.wasm` - WebAssembly binary (~196KB)
- `pedaler_core.js` - ES module loader
- `pedaler_core.d.ts` - TypeScript definitions

### Basic Usage (TypeScript)

```typescript
import init, { WasmPedalSim } from 'pedaler_core';

// Initialize WASM (once)
await init();

// Create simulator
const circuit = `
  .input in
  .output out
  V_IN in 0 DC 0
  R1 in out 10k
  R2 out 0 10k
`;
const sim = new WasmPedalSim(circuit, 48000);

// Process audio (in AudioWorklet)
sim.process_block(inputBuffer, outputBuffer);
```

### Configuration

```typescript
// Custom Newton-Raphson settings for performance tuning
const sim = WasmPedalSim.with_config(
  circuit,
  48000,  // sample rate
  50,     // max iterations
  1e-3    // tolerance (higher = faster, less precise)
);
```

### Full Documentation

For comprehensive integration guides including:
- Complete TypeScript/AudioWorklet integration
- Vite/Webpack configuration
- Performance optimization
- Troubleshooting

See **[docs/wasm_integration.md](docs/wasm_integration.md)**

---

## Technical Details

> [!NOTE]
> **📖 Deep Dive:** See [docs/architecture.md](docs/architecture.md) for internal architecture, module structure, and implementation details.

### Simulation Method

Pedaler uses **Modified Nodal Analysis (MNA)** to solve circuit equations at each audio sample:

1. Build system matrix **A** and source vector **z**
2. Solve **Ax = z** using LU decomposition with partial pivoting
3. For nonlinear elements, iterate with Newton-Raphson until convergence

### Discretization

Reactive elements (C, L) use **trapezoidal integration** for accurate frequency response:

- Capacitor companion model: `I = (2C/dt) * V + I_history`
- Inductor companion model: `V = (2L/dt) * I + V_history`

### Nonlinear Solving

- **Convergence tolerance**: 1e-6
- **Maximum iterations**: Configurable (Default: 50)
- **Voltage limiting**: Prevents numerical overflow in exp() functions

### Digital Effects Integration

Digital effects (DELAY, REVERB) are integrated directly into the MNA matrix as voltage sources:

1. Effects read their input node voltage from the previous sample's solution
2. Process the signal through delay buffer / FDN reverb
3. Stamp as voltage source driving the output node
4. MNA solve includes the effect as part of the circuit

This allows digital effects to be placed **anywhere** in the circuit topology, not just as post-processing. There is an inherent 1-sample latency (similar to capacitor companion models).

### LFO Modulation

LFO-modulated components update their values before each MNA solve:

1. All LFOs advance their phase by `rate / sample_rate`
2. LFO output value computed based on waveform shape (0-1 range)
3. Modulated resistors update: `R_eff = R_base * (1 + depth * range * lfo_value)`
4. MNA matrix is re-stamped with new resistance values

This "analog-style" modulation creates authentic phaser/flanger effects with swept filter notches.

## Documentation

| Document | Description |
|----------|-------------|
| [docs/dsl_reference.md](docs/dsl_reference.md) | Complete DSL syntax, grammar, and examples |
| [docs/components.md](docs/components.md) | Component models, physics, and parameters |
| [docs/architecture.md](docs/architecture.md) | Internal architecture and algorithms |
| [docs/wasm_integration.md](docs/wasm_integration.md) | WASM/Web integration guide |

## License

MIT License - see [LICENSE](./LICENSE) file for details.
