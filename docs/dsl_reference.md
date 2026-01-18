# Pedaler DSL Reference

Complete syntax reference for the Pedaler circuit description language.

## Table of Contents

1. [Overview](#overview)
2. [Basic Syntax](#basic-syntax)
3. [Value Notation](#value-notation)
4. [Components](#components)
5. [Directives](#directives)
6. [Models](#models)
7. [Digital Effects](#digital-effects)
8. [LFO and Modulation](#lfo-and-modulation)
9. [Examples](#examples)

---

## Overview

Pedaler DSL (`.ped` files) is a SPICE-inspired text format for describing analog and digital audio circuits. Each file defines a complete circuit with:

- Component declarations (resistors, capacitors, diodes, etc.)
- Node connections
- Component models (diode characteristics, transistor parameters)
- Input/output designation
- Optional digital effects and LFO modulation

### File Structure

```text
# Comments start with # or ;
; This is also a comment

# Directives
.input <node>
.output <node>
.model <name> <type> <params>

# Components
<prefix><name> <node1> <node2> [node3...] <value_or_model> [params]
```

---

## Basic Syntax

### Line Format

- **One statement per line** (no multi-line statements)
- **Whitespace-delimited** tokens
- **Case-insensitive** for keywords and prefixes
- **Case-sensitive** for node names and model names

### Comments

```text
# This is a comment
; This is also a comment (SPICE style)
R1 in out 10k  # Inline comments work too
```

### Node Names

- Any alphanumeric string: `in`, `out`, `n1`, `vcc`, `base`
- Ground node: `0` or `GND` (both refer to ground)
- Node names are **case-sensitive**: `In` ≠ `in`

### Reserved Names

| Name | Purpose |
|------|---------|
| `0` | Ground reference |
| `GND` | Ground reference (alias for 0) |
| `V_IN` | Recommended name for input voltage source |

---

## Value Notation

### Engineering Notation

Values support standard engineering prefixes:

| Suffix | Multiplier | Name |
|--------|------------|------|
| `p` | 10⁻¹² | pico |
| `n` | 10⁻⁹ | nano |
| `u` | 10⁻⁶ | micro |
| `m` | 10⁻³ | milli |
| `k` | 10³ | kilo |
| `M` | 10⁶ | mega |
| `G` | 10⁹ | giga |

### Examples

```text
10k     = 10,000 Ω
4.7k    = 4,700 Ω
100n    = 100 nF (0.0000001 F)
10u     = 10 µF
1M      = 1,000,000 Ω
2.2p    = 2.2 pF
```

### Scientific Notation

Standard scientific notation is also supported:

```text
1e-9    = 0.000000001
1.5e3   = 1500
2.52e-9 = 0.00000000252
```

---

## Components

### Resistor (R)

```text
R<name> <n+> <n-> <value>
```

| Parameter | Description |
|-----------|-------------|
| `n+` | Positive node |
| `n-` | Negative node |
| `value` | Resistance in ohms |

**Examples:**
```text
R1 in out 10k         # 10kΩ resistor
R_BIAS vcc base 100k  # 100kΩ bias resistor
RLOAD out 0 8         # 8Ω load (speaker)
```

### Capacitor (C)

```text
C<name> <n+> <n-> <value>
```

| Parameter | Description |
|-----------|-------------|
| `n+` | Positive node |
| `n-` | Negative node |
| `value` | Capacitance in farads |

**Examples:**
```text
C1 in out 100n        # 100nF coupling capacitor
C_BYPASS vcc 0 10u    # 10µF bypass capacitor
CFILTER out 0 47p     # 47pF filter cap
```

### Inductor (L)

```text
L<name> <n+> <n-> <value>
```

| Parameter | Description |
|-----------|-------------|
| `n+` | Positive node |
| `n-` | Negative node |
| `value` | Inductance in henries |

**Examples:**
```text
L1 in out 10m         # 10mH inductor
L_CHOKE vcc filt 100u # 100µH choke
```

### Diode (D)

```text
D<name> <anode> <cathode> <model>
```

| Parameter | Description |
|-----------|-------------|
| `anode` | Anode (positive) node |
| `cathode` | Cathode (negative) node |
| `model` | Model name (defined with `.model`) |

**Examples:**
```text
D1 in out 1N4148      # Signal diode
D_CLIP n1 0 DCLIP     # Clipping diode
D_LED out 0 LED_RED   # LED
```

### BJT Transistor (Q)

```text
Q<name> <collector> <base> <emitter> <model>
```

| Parameter | Description |
|-----------|-------------|
| `collector` | Collector node |
| `base` | Base node |
| `emitter` | Emitter node |
| `model` | Model name (NPN or PNP type) |

**Examples:**
```text
Q1 vcc base 0 2N3904      # NPN transistor
Q_PNP out base vcc 2N3906 # PNP transistor
```

### Voltage Source (V)

```text
V<name> <n+> <n-> <type> [value]
```

| Parameter | Description |
|-----------|-------------|
| `n+` | Positive node |
| `n-` | Negative node |
| `type` | `DC` or `AC` |
| `value` | Voltage value (optional for AC input) |

**Source Types:**
- `DC <value>` - Constant DC voltage
- `AC` - Audio input signal (value set by simulator)

**Examples:**
```text
V_IN in 0 AC          # Audio input source (required)
V_BIAS bias 0 DC 4.5  # 4.5V bias voltage
VCC vcc 0 DC 9        # 9V power supply
```

**Important:** Every circuit must have a voltage source named `V_IN` at the input node for audio signal injection.

### Current Source (I)

```text
I<name> <n+> <n-> <value>
```

| Parameter | Description |
|-----------|-------------|
| `n+` | Current flows into this node |
| `n-` | Current flows out of this node |
| `value` | Current in amperes |

**Examples:**
```text
I1 vcc collector 1m   # 1mA current source
I_BIAS 0 base 10u     # 10µA bias current
```

### Op-Amp (OP)

```text
OP<name> <n+> <n-> <out> <model>
```

| Parameter | Description |
|-----------|-------------|
| `n+` | Non-inverting input |
| `n-` | Inverting input |
| `out` | Output node |
| `model` | Model name |

**Examples:**
```text
OP1 np nm out IDEAL   # Ideal op-amp
OP_TL072 in_p in_m out TL072
```

### Potentiometer (POT)

```text
POT<name> <n1> <wiper> <n2> <value> <position>
```

| Parameter | Description |
|-----------|-------------|
| `n1` | First terminal |
| `wiper` | Wiper (variable tap) |
| `n2` | Second terminal |
| `value` | Total resistance |
| `position` | Wiper position (0.0 to 1.0) |

**Examples:**
```text
POT1 in wiper out 100k 0.5   # 50% position
POT_VOL in tap 0 10k 0.75    # 75% volume
```

### Switch (SW)

```text
SW<name> <n1> <n2> <state>
```

| Parameter | Description |
|-----------|-------------|
| `n1` | First terminal |
| `n2` | Second terminal |
| `state` | `OPEN` or `CLOSED` |

**Examples:**
```text
SW1 in out CLOSED     # Closed switch
SW_BYPASS in bypass OPEN
```

---

## Directives

### Input Declaration

```text
.input <node>
```

Marks the node where audio signal enters the circuit. **Required.**

```text
.input in
```

### Output Declaration

```text
.output <node>
```

Marks the node where processed audio is read. **Required.**

```text
.output out
```

### Model Definition

```text
.model <name> <type> <param1>=<value1> <param2>=<value2> ...
```

or (SPICE-compatible):

```text
.model <name> <type> (<param1>=<value1> <param2>=<value2> ...)
```

**Model Types:**
| Type | Component |
|------|-----------|
| `D` | Diode |
| `NPN` | NPN BJT |
| `PNP` | PNP BJT |
| `OP` | Op-Amp |

**Examples:**
```text
.model 1N4148 D IS=2.52e-9 N=1.752 VT=0.026
.model DCLIP D (vf=0.3 is=1e-9 n=1.8)
.model 2N3904 NPN bf=100 is=1e-14
.model IDEAL OP gain=1e6
```

---

## Models

### Diode Model Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `is` | Saturation current (A) | 1e-14 |
| `n` | Ideality factor | 1.0 |
| `vf` | Forward voltage (V) | 0.7 |

**Common Diode Models:**
```text
# Silicon signal diode
.model 1N4148 D IS=2.52e-9 N=1.752

# Germanium diode (lower Vf)
.model 1N34A D IS=1e-9 N=1.5 VF=0.3

# Silicon clipper (soft clipping)
.model DCLIP D IS=1e-9 N=1.8 VF=0.3

# LED (red)
.model LED_RED D IS=1e-18 N=2.0 VF=1.8
```

### BJT Model Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `bf` | Forward beta (current gain) | 100 |
| `br` | Reverse beta | 1 |
| `is` | Saturation current (A) | 1e-14 |
| `n` | Ideality factor | 1.0 |
| `va` | Early voltage (V) | 100 |

**Common BJT Models:**
```text
# General purpose NPN
.model 2N3904 NPN BF=100 IS=1e-14

# High gain NPN
.model BC549C NPN BF=500 IS=1e-14

# General purpose PNP
.model 2N3906 PNP BF=100 IS=1e-14
```

### Op-Amp Model Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `gain` | Open-loop gain | 1e6 (ideal) |
| `rin` | Input resistance (Ω) | 1e12 |
| `rout` | Output resistance (Ω) | 0.01 |

**Common Op-Amp Models:**
```text
# Ideal op-amp (infinite gain, no saturation)
.model IDEAL OP GAIN=1e6

# TL072 (JFET input)
.model TL072 OP GAIN=200000 RIN=1e12 ROUT=100

# LM741 (bipolar input)
.model LM741 OP GAIN=200000 RIN=2e6 ROUT=75
```

---

## Digital Effects

Digital effects are integrated as in-circuit components, not post-processing.

### Delay Line (DELAY)

```text
DELAY <name> <in_node> <out_node> <time> [mix=<value>] [feedback=<value>]
```

| Parameter | Description | Default |
|-----------|-------------|---------|
| `time` | Delay time (seconds or with suffix) | Required |
| `mix` | Dry/wet mix (0.0-1.0) | 0.5 |
| `feedback` | Feedback amount (0.0-1.0) | 0.0 |

**Examples:**
```text
DELAY d1 in out 300m                    # 300ms delay, 50% mix
DELAY d1 in out 0.5 mix=0.3             # 500ms delay, 30% wet
DELAY d1 in out 100m mix=0.5 feedback=0.4  # Echo with feedback
```

### FDN Reverb (REVERB)

```text
REVERB <name> <in_node> <out_node> [decay=<v>] [size=<v>] [damping=<v>] [mix=<v>] [predelay=<v>]
```

| Parameter | Description | Default |
|-----------|-------------|---------|
| `decay` | Reverb tail decay (0.0-1.0) | 0.5 |
| `size` | Room size (0.0-1.0) | 0.5 |
| `damping` | High-frequency damping (0.0-1.0) | 0.3 |
| `mix` | Dry/wet mix (0.0-1.0) | 0.5 |
| `predelay` | Initial delay before reverb (seconds) | 0 |

**Examples:**
```text
REVERB r1 in out                        # Default reverb
REVERB r1 in out decay=0.8 size=0.7     # Large room
REVERB r1 in out decay=0.3 size=0.2     # Small room
REVERB r1 in out decay=0.9 damping=0.5 mix=0.6  # Hall reverb
```

---

## LFO and Modulation

### LFO Declaration

```text
LFO <name> <rate> <shape>
```

| Parameter | Description |
|-----------|-------------|
| `name` | LFO identifier |
| `rate` | Oscillation frequency in Hz |
| `shape` | Waveform type |

**Waveform Shapes:**
| Shape | Description | Output Range |
|-------|-------------|--------------|
| `sine` | Smooth sine wave | 0.0 to 1.0 |
| `triangle` | Linear triangle wave | 0.0 to 1.0 |
| `sawtooth` | Rising sawtooth | 0.0 to 1.0 |
| `square` | Square wave | 0.0 or 1.0 |

**Examples:**
```text
LFO lfo1 0.5 sine       # 0.5 Hz sine wave
LFO lfo2 2.0 triangle   # 2 Hz triangle wave
LFO lfo3 4.0 square     # 4 Hz square wave
```

### Modulated Resistor

```text
R<name> <n1> <n2> <base_value> <lfo_name> [depth=<v>] [range=<v>]
```

| Parameter | Description | Default |
|-----------|-------------|---------|
| `lfo_name` | Name of LFO to use | Required |
| `depth` | Modulation depth (0.0-1.0) | 0.8 |
| `range` | Modulation range multiplier | 4.0 |

**Modulation Formula:**
```
R_effective = R_base × (1 + depth × range × lfo_value)
```

Where `lfo_value` oscillates between 0.0 and 1.0.

**Examples:**
```text
LFO lfo1 0.5 sine
R_MOD n1 n2 10k lfo1              # Default depth=0.8, range=4.0
R_MOD n1 n2 10k lfo1 depth=0.5    # 50% modulation depth
R_MOD n1 n2 10k lfo1 depth=0.8 range=2.0  # Custom range
```

---

## Examples

### Minimal Valid Circuit

```text
.input in
.output out

V_IN in 0 DC 0
R1 in out 10k
R2 out 0 10k
```

### RC Low-Pass Filter

```text
# First-order RC low-pass filter
# Cutoff frequency: fc = 1/(2πRC) ≈ 1.59 kHz

.input in
.output out

V_IN in 0 AC
R1 in out 10k
C1 out 0 10n
```

### Diode Clipper

```text
# Symmetrical hard clipper with silicon diodes

.input in
.output out

V_IN in 0 AC
R_IN in clip 4.7k
D1 clip 0 DCLIP
D2 0 clip DCLIP
R_OUT clip out 10k

.model DCLIP D IS=1e-9 N=1.8 VF=0.3
```

### Op-Amp Overdrive (Tube Screamer Style)

```text
# Tube Screamer-style op-amp clipping stage

.input in
.output out

# Input coupling
V_IN in 0 AC
C_IN in buf 47n
R_BIAS1 buf 0 510k

# Op-amp gain stage
OP1 vref fb out IDEAL
R_GAIN buf fb 4.7k
D1 fb out DCLIP
D2 out fb DCLIP

# Virtual ground
V_BIAS vref 0 DC 4.5

# Output
C_OUT out final 100n
R_OUT final 0 10k

.output final

.model DCLIP D IS=1e-9 N=1.8 VF=0.3
.model IDEAL OP GAIN=1e6
```

### 4-Stage Phaser

```text
# 4-stage all-pass phaser with LFO modulation

.input in
.output out

LFO lfo1 0.5 sine

V_IN in 0 AC

# Stage 1
R1 in ap1 10k lfo1 depth=0.8 range=4.0
C1 ap1 0 10n

# Stage 2
R2 ap1 ap2 10k lfo1 depth=0.8 range=4.0
C2 ap2 0 10n

# Stage 3
R3 ap2 ap3 10k lfo1 depth=0.8 range=4.0
C3 ap3 0 10n

# Stage 4
R4 ap3 out 10k lfo1 depth=0.8 range=4.0
C4 out 0 10n
```

### Delay with Distortion

```text
# Distortion into delay chain

.input in
.output out

V_IN in 0 AC

# Distortion stage
R1 in clip 4.7k
D1 clip 0 DCLIP
D2 0 clip DCLIP

# Delay
DELAY d1 clip out 300m mix=0.4 feedback=0.3

.model DCLIP D IS=1e-9 N=1.8 VF=0.3
```

---

## Grammar Summary (BNF-like)

```bnf
circuit     ::= (line)*
line        ::= (component | directive | comment | empty) NEWLINE
comment     ::= ('#' | ';') TEXT
directive   ::= input_dir | output_dir | model_dir
input_dir   ::= '.input' NODE
output_dir  ::= '.output' NODE
model_dir   ::= '.model' NAME TYPE params
params      ::= (NAME '=' VALUE)*
component   ::= resistor | capacitor | inductor | diode | bjt | vsource | isource | opamp | pot | switch | delay | reverb | lfo
resistor    ::= 'R' NAME NODE NODE VALUE [NAME params]
capacitor   ::= 'C' NAME NODE NODE VALUE
inductor    ::= 'L' NAME NODE NODE VALUE
diode       ::= 'D' NAME NODE NODE NAME
bjt         ::= 'Q' NAME NODE NODE NODE NAME
vsource     ::= 'V' NAME NODE NODE ('DC' VALUE | 'AC')
isource     ::= 'I' NAME NODE NODE VALUE
opamp       ::= 'OP' NAME NODE NODE NODE NAME
pot         ::= 'POT' NAME NODE NODE NODE VALUE VALUE
switch      ::= 'SW' NAME NODE NODE ('OPEN' | 'CLOSED')
delay       ::= 'DELAY' NAME NODE NODE VALUE params
reverb      ::= 'REVERB' NAME NODE NODE params
lfo         ::= 'LFO' NAME VALUE SHAPE

NODE        ::= [a-zA-Z_][a-zA-Z0-9_]* | '0' | 'GND'
NAME        ::= [a-zA-Z_][a-zA-Z0-9_]*
VALUE       ::= NUMBER [SUFFIX]
NUMBER      ::= [0-9]+ ('.' [0-9]+)? ('e' [+-]? [0-9]+)?
SUFFIX      ::= 'p' | 'n' | 'u' | 'm' | 'k' | 'M' | 'G'
TYPE        ::= 'D' | 'NPN' | 'PNP' | 'OP'
SHAPE       ::= 'sine' | 'triangle' | 'sawtooth' | 'square'
```

---

## See Also

- [Component Models](./components.md) - Detailed component physics and parameters
- [Architecture Overview](./architecture.md) - How the simulator works internally
- [WASM Integration](./wasm_integration.md) - Using Pedaler in web applications
