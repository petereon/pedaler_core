# Component Models

Detailed documentation of all supported circuit components, their physics, mathematical models, and implementation details.

## Table of Contents

1. [Linear Components](#linear-components)
2. [Voltage and Current Sources](#voltage-and-current-sources)
3. [Nonlinear Components](#nonlinear-components)
4. [Control Components](#control-components)
5. [Digital Effects](#digital-effects)
6. [Modulation](#modulation)

---

## Linear Components

### Resistor

**Symbol Prefix:** `R`

**Physics:**
Ohm's Law relates voltage and current: $V = IR$

**MNA Stamping:**
A resistor with conductance $G = 1/R$ between nodes $n_1$ and $n_2$ is stamped as:

| | $n_1$ | $n_2$ |
|---|---|---|
| $n_1$ | $+G$ | $-G$ |
| $n_2$ | $-G$ | $+G$ |

**Parameters:**
| Parameter | Type | Unit | Description |
|-----------|------|------|-------------|
| `value` | f64 | Ω | Resistance value |

**Modulation:**
Resistors can be modulated by an LFO for time-varying effects:

$$R_{eff}(t) = R_{base} \times (1 + \text{depth} \times \text{range} \times \text{lfo}(t))$$

**DSL Example:**
```text
R1 in out 10k
R_MOD n1 n2 10k lfo1 depth=0.8 range=4.0
```

---

### Capacitor

**Symbol Prefix:** `C`

**Physics:**
The capacitor's current-voltage relationship is: $I = C \frac{dV}{dt}$

**Discretization:**
Using the trapezoidal rule for improved frequency response accuracy:

$$I_n = \frac{2C}{\Delta t}(V_n - V_{n-1}) + I_{n-1}$$

This yields a companion model with:
- Equivalent conductance: $G_{eq} = \frac{2C}{\Delta t}$
- History current source: $I_{eq} = \frac{2C}{\Delta t} V_{n-1} + I_{n-1}$

**MNA Stamping:**
Same as resistor with $G = G_{eq}$, plus a current source $I_{eq}$ in the source vector.

**Parameters:**
| Parameter | Type | Unit | Description |
|-----------|------|------|-------------|
| `value` | f64 | F | Capacitance value |

**State Variables:**
- `v_prev`: Previous voltage across capacitor
- `i_prev`: Previous current through capacitor

**DSL Example:**
```text
C1 in out 100n
C_BYPASS vcc 0 10u
```

---

### Inductor

**Symbol Prefix:** `L`

**Physics:**
The inductor's voltage-current relationship is: $V = L \frac{dI}{dt}$

**Discretization:**
Using the trapezoidal rule:

$$V_n = \frac{2L}{\Delta t}(I_n - I_{n-1}) + V_{n-1}$$

Companion model:
- Equivalent resistance: $R_{eq} = \frac{2L}{\Delta t}$
- History voltage source: $V_{eq} = \frac{2L}{\Delta t} I_{n-1} + V_{n-1}$

**MNA Stamping:**
Requires an extra branch current variable. Stamped as a voltage source with series resistance.

**Parameters:**
| Parameter | Type | Unit | Description |
|-----------|------|------|-------------|
| `value` | f64 | H | Inductance value |

**State Variables:**
- `i_prev`: Previous current through inductor
- `v_prev`: Previous voltage across inductor

**DSL Example:**
```text
L1 in out 10m
L_CHOKE vcc filt 100u
```

---

## Voltage and Current Sources

### Voltage Source

**Symbol Prefix:** `V`

**Physics:**
Ideal voltage source maintains constant voltage regardless of current.

**MNA Stamping:**
Voltage sources require an additional branch current variable. For a voltage source $V_s$ between nodes $n_+$ and $n_-$ with branch index $i_b$:

| | $n_+$ | $n_-$ | $i_b$ | RHS |
|---|---|---|---|---|
| $n_+$ | | | $+1$ | |
| $n_-$ | | | $-1$ | |
| $i_b$ | $+1$ | $-1$ | | $V_s$ |

**Source Types:**

| Type | Description | Usage |
|------|-------------|-------|
| `DC` | Constant voltage | Bias, power supplies |
| `AC` | Time-varying input | Audio signal injection |

For `AC` sources, the simulator sets the voltage value from the input audio sample each time step.

**Parameters:**
| Parameter | Type | Unit | Description |
|-----------|------|------|-------------|
| `dc_value` | f64 | V | DC voltage (for DC type) |
| `ac_value` | f64 | V | Instantaneous AC voltage (set by simulator) |

**DSL Example:**
```text
V_IN in 0 AC            # Audio input
V_BIAS bias 0 DC 4.5    # DC bias
VCC vcc 0 DC 9          # Power supply
```

---

### Current Source

**Symbol Prefix:** `I`

**Physics:**
Ideal current source maintains constant current regardless of voltage.

**MNA Stamping:**
Current sources only affect the source vector (RHS):

| | RHS |
|---|---|
| $n_+$ | $-I_s$ |
| $n_-$ | $+I_s$ |

(Current flows from $n_+$ to $n_-$)

**Parameters:**
| Parameter | Type | Unit | Description |
|-----------|------|------|-------------|
| `dc_value` | f64 | A | Source current |

**DSL Example:**
```text
I1 vcc collector 1m
I_BIAS 0 base 10u
```

---

## Nonlinear Components

### Diode

**Symbol Prefix:** `D`

**Physics:**
The Shockley diode equation describes the I-V characteristic:

$$I = I_s \left( e^{\frac{V}{nV_T}} - 1 \right)$$

Where:
- $I_s$ = Saturation current (typically 10⁻¹⁴ to 10⁻⁹ A)
- $n$ = Ideality factor (1.0 to 2.0)
- $V_T$ = Thermal voltage (≈26mV at room temperature)

**Newton-Raphson Linearization:**
For iterative solving, the diode is linearized around operating point $V_0$:

$$I \approx I_0 + G_d(V - V_0)$$

Where the dynamic conductance is:

$$G_d = \frac{dI}{dV}\bigg|_{V_0} = \frac{I_s}{nV_T} e^{\frac{V_0}{nV_T}}$$

**Voltage Limiting:**
To prevent numerical overflow from $e^{V/(nV_T)}$, voltage is limited:

$$V_{limited} = V_{crit} + nV_T \ln\left(1 + \frac{V - V_{crit}}{nV_T}\right) \quad \text{for } V > V_{crit}$$

**Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `is` | f64 | 1e-14 | Saturation current (A) |
| `n` | f64 | 1.0 | Ideality factor |
| `vf` | f64 | 0.7 | Forward voltage (V) |

**Common Models:**

| Model | Is | n | Vf | Use Case |
|-------|----|----|-----|----------|
| 1N4148 | 2.52e-9 | 1.752 | 0.7 | Signal clipping |
| 1N34A | 1e-9 | 1.5 | 0.3 | Germanium, soft clip |
| LED | 1e-18 | 2.0 | 1.8-3.3 | Indicator |

**DSL Example:**
```text
D1 anode cathode 1N4148
.model 1N4148 D IS=2.52e-9 N=1.752
```

---

### BJT (Bipolar Junction Transistor)

**Symbol Prefix:** `Q`

**Physics:**
The simplified Ebers-Moll model treats the BJT as two diodes with a current-controlled current source.

**NPN Equations:**
$$I_C = I_S \left( e^{\frac{V_{BE}}{nV_T}} - e^{\frac{V_{BC}}{nV_T}} \right) - \frac{I_S}{\beta_R} \left( e^{\frac{V_{BC}}{nV_T}} - 1 \right)$$

$$I_B = \frac{I_S}{\beta_F} \left( e^{\frac{V_{BE}}{nV_T}} - 1 \right) + \frac{I_S}{\beta_R} \left( e^{\frac{V_{BC}}{nV_T}} - 1 \right)$$

$$I_E = I_C + I_B$$

For PNP, voltage polarities are reversed.

**Newton-Raphson:**
The BJT stamps a 3x3 Jacobian into the MNA matrix, linearizing both junction diodes.

**Voltage Limiting:**
Both $V_{BE}$ and $V_{BC}$ are limited to prevent overflow, similar to diode limiting.

**Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bf` | f64 | 100 | Forward beta (current gain) |
| `br` | f64 | 1 | Reverse beta |
| `is` | f64 | 1e-14 | Saturation current (A) |
| `n` | f64 | 1.0 | Ideality factor |
| `va` | f64 | 100 | Early voltage (V) |

**Terminal Order:** Collector, Base, Emitter

**DSL Example:**
```text
Q1 vcc base 0 2N3904
.model 2N3904 NPN BF=100 IS=1e-14
```

---

### Op-Amp (Operational Amplifier)

**Symbol Prefix:** `OP`

**Physics:**
The op-amp is modeled with:
- Differential input with high input impedance
- Voltage-controlled voltage source output
- Optional output resistance

**Ideal Model:**
$$V_{out} = A_{OL}(V_+ - V_-)$$

Where $A_{OL}$ is the open-loop gain (typically 10⁵ to 10⁶ for ideal model).

With negative feedback, the virtual short approximation applies:
$$V_+ \approx V_-$$

**MNA Stamping:**
The op-amp requires branch current variables for proper formulation:
1. Input stage: High-resistance between inputs
2. Output stage: VCVS (Voltage-Controlled Voltage Source)

**Rail Limiting:**
Output is clamped to power supply rails (if specified):
$$V_{out} = \text{clamp}(V_{out}, V_{rail-}, V_{rail+})$$

**Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `gain` | f64 | 1e6 | Open-loop gain |
| `rin` | f64 | 1e12 | Input resistance (Ω) |
| `rout` | f64 | 0.01 | Output resistance (Ω) |

**Terminal Order:** Non-inverting input, Inverting input, Output

**DSL Example:**
```text
OP1 np nm out IDEAL
.model IDEAL OP GAIN=1e6
```

---

## Control Components

### Potentiometer

**Symbol Prefix:** `POT`

**Model:**
A potentiometer is modeled as two resistors in series:

```
n1 ---[R1]--- wiper ---[R2]--- n2
```

Where:
- $R_1 = R_{total} \times position$
- $R_2 = R_{total} \times (1 - position)$

**Parameters:**
| Parameter | Type | Unit | Description |
|-----------|------|------|-------------|
| `value` | f64 | Ω | Total resistance |
| `position` | f64 | - | Wiper position (0.0 to 1.0) |

**DSL Example:**
```text
POT1 in wiper out 100k 0.5
```

---

### Switch

**Symbol Prefix:** `SW`

**Model:**
A switch is a resistor with state-dependent resistance:
- **CLOSED:** Very low resistance (0.001Ω)
- **OPEN:** Very high resistance (1MΩ)

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `state` | enum | `OPEN` or `CLOSED` |

**DSL Example:**
```text
SW1 in out CLOSED
SW_BYPASS effect_in effect_out OPEN
```

---

## Digital Effects

Digital effects are implemented as in-circuit voltage sources with 1-sample latency.

### Delay Line

**Symbol Prefix:** `DELAY`

**Algorithm:**
1. Read input node voltage from previous sample's solution
2. Write to circular buffer
3. Read from buffer at delay offset
4. Apply mix and feedback
5. Stamp as voltage source driving output node

**Signal Flow:**
$$y[n] = (1 - mix) \cdot x[n] + mix \cdot (buffer[n - delay] + feedback \cdot y[n-1])$$

**Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `time` | f64 | - | Delay time (seconds) |
| `mix` | f64 | 0.5 | Dry/wet mix (0.0-1.0) |
| `feedback` | f64 | 0.0 | Feedback amount (0.0-1.0) |

**Implementation Notes:**
- Buffer size: `sample_rate × max_delay_time`
- Interpolation: Linear (for fractional delays)
- Latency: 1 sample (inherent to MNA integration)

**DSL Example:**
```text
DELAY d1 in out 300m mix=0.5 feedback=0.4
```

---

### FDN Reverb

**Symbol Prefix:** `REVERB`

**Algorithm:**
Feedback Delay Network with 4 delay lines and Hadamard mixing matrix:

1. Input fed to all delay lines
2. Each line has unique prime-number delay length
3. Outputs mixed through Hadamard matrix
4. Damping filters applied (one-pole lowpass)
5. Fed back with decay coefficient

**Hadamard Matrix (4x4):**
$$H = \frac{1}{2}\begin{bmatrix} 1 & 1 & 1 & 1 \\ 1 & -1 & 1 & -1 \\ 1 & 1 & -1 & -1 \\ 1 & -1 & -1 & 1 \end{bmatrix}$$

**Delay Line Lengths (at 48kHz):**
Base delays scaled by `size` parameter:
- Line 1: 1087 samples
- Line 2: 1283 samples
- Line 3: 1511 samples
- Line 4: 1777 samples

**Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `decay` | f64 | 0.5 | Reverb tail decay (0.0-1.0) |
| `size` | f64 | 0.5 | Room size scaling (0.0-1.0) |
| `damping` | f64 | 0.3 | High-frequency damping (0.0-1.0) |
| `mix` | f64 | 0.5 | Dry/wet mix (0.0-1.0) |
| `predelay` | f64 | 0.0 | Initial delay (seconds) |

**DSL Example:**
```text
REVERB r1 in out decay=0.7 size=0.6 damping=0.4 mix=0.5
```

---

## Modulation

### LFO (Low Frequency Oscillator)

**Symbol Prefix:** `LFO`

**Purpose:**
Generate low-frequency control signals for modulating component values.

**Waveforms:**

| Shape | Formula | Output Range |
|-------|---------|--------------|
| `sine` | $\frac{1}{2}(1 + \sin(2\pi f t))$ | 0.0 to 1.0 |
| `triangle` | $2 \| 2(ft \mod 1) - 1 \| - 1$ | 0.0 to 1.0 |
| `sawtooth` | $ft \mod 1$ | 0.0 to 1.0 |
| `square` | $(ft \mod 1) < 0.5$ ? 1 : 0 | 0.0 or 1.0 |

**Parameters:**
| Parameter | Type | Unit | Description |
|-----------|------|------|-------------|
| `rate` | f64 | Hz | Oscillation frequency |
| `shape` | enum | - | Waveform type |

**Phase Accumulation:**
$$\phi_{n+1} = (\phi_n + \frac{f}{f_s}) \mod 1$$

Where $f$ is the LFO rate and $f_s$ is the sample rate.

**Usage:**
LFOs modulate resistor values for phaser/flanger effects:

$$R_{eff}(t) = R_{base} \times (1 + depth \times range \times lfo(t))$$

**DSL Example:**
```text
LFO lfo1 0.5 sine
R_MOD n1 n2 10k lfo1 depth=0.8 range=4.0
```

---

## Component Summary Table

| Prefix | Component | Nodes | Linear | Requires Model |
|--------|-----------|-------|--------|----------------|
| `R` | Resistor | 2 | Yes | No |
| `C` | Capacitor | 2 | Yes | No |
| `L` | Inductor | 2 | Yes | No |
| `V` | Voltage Source | 2 | Yes | No |
| `I` | Current Source | 2 | Yes | No |
| `D` | Diode | 2 | No | Yes |
| `Q` | BJT | 3 | No | Yes |
| `OP` | Op-Amp | 3 | No* | Yes |
| `POT` | Potentiometer | 3 | Yes | No |
| `SW` | Switch | 2 | Yes | No |
| `DELAY` | Delay Line | 2 | N/A | No |
| `REVERB` | FDN Reverb | 2 | N/A | No |
| `LFO` | LFO | 0 | N/A | No |

*Op-amp uses quasi-linear model with limiting

---

## See Also

- [DSL Reference](./dsl_reference.md) - Complete syntax documentation
- [Architecture Overview](./architecture.md) - Simulation algorithm details
- [WASM Integration](./wasm_integration.md) - Web application usage
