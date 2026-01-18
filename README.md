# Pedaler Core

A real-time circuit simulator for guitar pedals written in Rust.

Pedaler simulates analog guitar effect circuits at audio sample rate, allowing you to process audio through virtual pedal circuits defined in a simple text-based DSL.

## Features

- **Real-time simulation** using Modified Nodal Analysis (MNA)
- **Linear components**: Resistors, Capacitors, Inductors
- **Nonlinear components**: Diodes, BJTs, Op-Amps
- **Control elements**: Potentiometers, Switches
- **Sources**: DC/AC Voltage sources, Current sources
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
| `-h, --help` | Print help information | |
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

The `examples/` directory contains ready-to-use circuit files:

| File | Description |
|------|-------------|
| `rc_lowpass.ped` | First-order RC low-pass filter (fc ≈ 1.59 kHz) |
| `diode_clipper.ped` | Symmetrical diode hard clipper |
| `opamp_overdrive.ped` | Tube Screamer-style op-amp overdrive |

## Technical Details

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
- **Maximum iterations**: 50
- **Voltage limiting**: Prevents numerical overflow in exp() functions

## License

MIT License - see LICENSE file for details.
