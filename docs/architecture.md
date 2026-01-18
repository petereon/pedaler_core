# Architecture Overview

This document describes the internal architecture of Pedaler Core, explaining how the circuit simulator works from DSL parsing to audio output.

## Table of Contents

1. [System Overview](#system-overview)
2. [Module Structure](#module-structure)
3. [Data Flow](#data-flow)
4. [DSL Parsing](#dsl-parsing)
5. [Circuit Representation](#circuit-representation)
6. [MNA Solver](#mna-solver)
7. [Newton-Raphson Iteration](#newton-raphson-iteration)
8. [Audio Processing Pipeline](#audio-processing-pipeline)
9. [Digital Effects Integration](#digital-effects-integration)
10. [LFO Modulation System](#lfo-modulation-system)
11. [WASM Architecture](#wasm-architecture)

---

## System Overview

Pedaler Core simulates analog circuits at audio sample rate using **Modified Nodal Analysis (MNA)**. The simulator:

1. Parses a circuit description (DSL)
2. Builds a circuit graph
3. Assembles the MNA matrix equation $Ax = z$
4. Solves for node voltages and branch currents
5. Handles nonlinear components via Newton-Raphson iteration
6. Processes audio sample-by-sample

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   DSL File  │────►│   Parser    │────►│   Circuit   │
│  (.ped)     │     │   (AST)     │     │   Graph     │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Audio     │◄────│  Simulator  │◄────│ MNA Matrix  │
│   Output    │     │   (step)    │     │   Solver    │
└─────────────┘     └─────────────┘     └─────────────┘
```

---

## Module Structure

```
src/
├── lib.rs              # Library entry point, re-exports
├── main.rs             # CLI binary entry point
├── error.rs            # Error types (PedalerError)
│
├── dsl/                # DSL parsing
│   ├── mod.rs          # Module exports
│   ├── lexer.rs        # Tokenization
│   ├── parser.rs       # AST construction
│   └── ast.rs          # AST type definitions
│
├── circuit/            # Circuit representation
│   ├── mod.rs          # Circuit struct, validation
│   └── types.rs        # NodeId, ComponentId, BranchId
│
├── components/         # Component models
│   ├── mod.rs          # Component enum, factory
│   ├── linear.rs       # R, C, L
│   ├── sources.rs      # V, I sources
│   ├── diode.rs        # Diode model
│   ├── bjt.rs          # BJT model
│   ├── opamp.rs        # Op-amp model
│   ├── controls.rs     # POT, SW
│   ├── delay.rs        # Delay line
│   ├── reverb.rs       # FDN reverb
│   └── lfo.rs          # LFO oscillator
│
├── solver/             # Numerical solving
│   ├── mod.rs          # Module exports
│   ├── mna.rs          # MNA matrix assembly
│   ├── newton.rs       # Newton-Raphson iteration
│   └── simulator.rs    # Main Simulator struct
│
├── audio/              # Audio I/O (CLI only)
│   └── mod.rs          # stdin/stdout PCM handling
│
└── wasm.rs             # WASM bindings (wasm feature)
```

### Feature Flags

| Feature | Dependencies | Purpose |
|---------|--------------|---------|
| `cli` (default) | `clap` | Command-line interface |
| `wasm` | `wasm-bindgen`, `console_error_panic_hook` | WebAssembly bindings |

---

## Data Flow

### Initialization Phase

```
DSL String
    │
    ▼
┌───────────┐
│  Lexer    │ ──► Token stream
└───────────┘
    │
    ▼
┌───────────┐
│  Parser   │ ──► AST (ComponentDef, ModelDef, directives)
└───────────┘
    │
    ▼
┌───────────┐
│ Circuit   │ ──► Circuit graph (nodes, components, models)
│ Builder   │
└───────────┘
    │
    ▼
┌───────────┐
│ Validator │ ──► Validated circuit (or error)
└───────────┘
    │
    ▼
┌───────────┐
│ Simulator │ ──► Ready to process audio
│   new()   │
└───────────┘
```

### Runtime Phase (per sample)

```
Input Sample (f32)
    │
    ▼
┌─────────────────┐
│ set_input()     │ ──► V_IN source voltage updated
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Update LFOs     │ ──► Advance LFO phases
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Update Modulated│ ──► Resistor values recalculated
│ Components      │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Process Digital │ ──► Delay/Reverb read input, compute output
│ Effects         │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Stamp Linear    │ ──► MNA matrix populated
│ Components      │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Newton-Raphson  │ ──► Nonlinear components solved iteratively
│ (if needed)     │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ LU Solve        │ ──► Node voltages computed
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Read Output     │ ──► V(output_node) returned
└─────────────────┘
    │
    ▼
Output Sample (f32)
```

---

## DSL Parsing

### Lexer

The lexer (`src/dsl/lexer.rs`) converts source text into tokens:

```rust
pub enum Token {
    Identifier(String),    // Component names, node names
    Number(f64),           // Values with engineering notation
    Directive(String),     // .input, .output, .model
    Equals,                // =
    OpenParen,             // (
    CloseParen,            // )
    Newline,
    Eof,
}
```

**Engineering Notation Handling:**
The lexer recognizes suffixes and applies multipliers:
```rust
fn parse_value(s: &str) -> f64 {
    // "10k" → 10000.0
    // "100n" → 0.0000001
}
```

### Parser

The parser (`src/dsl/parser.rs`) constructs an AST:

```rust
pub struct Ast {
    pub components: Vec<ComponentDef>,
    pub models: Vec<ModelDef>,
    pub input_node: Option<String>,
    pub output_node: Option<String>,
    pub lfos: Vec<LfoDef>,
}

pub struct ComponentDef {
    pub name: String,
    pub component_type: ComponentType,
    pub nodes: Vec<String>,
    pub value: Option<f64>,
    pub model_name: Option<String>,
    pub params: HashMap<String, f64>,
    pub line: usize,
}
```

### Error Handling

Parse errors include line/column information:
```rust
pub enum PedalerError {
    ParseError { line: usize, col: usize, message: String },
    UnknownComponent { name: String, line: usize },
    // ...
}
```

---

## Circuit Representation

### Node and Component IDs

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    pub const GROUND: NodeId = NodeId(0);
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ComponentId(pub usize);

#[derive(Copy, Clone)]
pub struct BranchId(pub usize);
```

### Circuit Structure

```rust
pub struct Circuit {
    /// All nodes except ground (ground is implicit)
    pub nodes: Vec<String>,
    /// Node name to NodeId mapping
    pub node_map: HashMap<String, NodeId>,
    /// All components
    pub components: Vec<Component>,
    /// Model definitions
    pub models: HashMap<String, ModelDef>,
    /// Input node (where V_IN connects)
    pub input_node: NodeId,
    /// Output node (where we read voltage)
    pub output_node: NodeId,
    /// LFO definitions
    pub lfos: Vec<LfoDef>,
    /// Digital delay definitions
    pub delays: Vec<DelayDef>,
    /// Digital reverb definitions
    pub reverbs: Vec<ReverbDef>,
    /// Number of branch current variables
    pub num_branches: usize,
}
```

### Validation

The `validate_circuit()` function checks:
1. Input and output nodes are declared
2. V_IN voltage source exists at input
3. No floating nodes (all nodes have path to ground)
4. All referenced models exist
5. Component values are valid (positive R, C, L)

---

## MNA Solver

### Modified Nodal Analysis

MNA formulates circuit equations as a linear system $Ax = z$ where:

- $A$ is the system matrix (conductances + voltage source constraints)
- $x$ is the solution vector (node voltages + branch currents)
- $z$ is the source vector (current sources + voltage source values)

### Matrix Structure

For a circuit with $n$ nodes (excluding ground) and $m$ voltage sources:

$$\begin{bmatrix} G & B \\ C & D \end{bmatrix} \begin{bmatrix} v \\ i \end{bmatrix} = \begin{bmatrix} i_s \\ v_s \end{bmatrix}$$

Where:
- $G$ ($n \times n$): Conductance matrix
- $B$ ($n \times m$): Voltage source incidence
- $C$ ($m \times n$): Transpose of B
- $D$ ($m \times m$): Usually zero (depends on dependent sources)
- $v$: Node voltages
- $i$: Branch currents through voltage sources
- $i_s$: Current source contributions
- $v_s$: Voltage source values

### Stamping

Each component "stamps" values into the matrix:

**Resistor (conductance G between nodes i, j):**
```rust
fn stamp_resistor(&mut self, n1: NodeId, n2: NodeId, g: f64) {
    // A[i,i] += G, A[j,j] += G
    // A[i,j] -= G, A[j,i] -= G
}
```

**Voltage Source (V between nodes, branch index b):**
```rust
fn stamp_voltage_source(&mut self, np: NodeId, nm: NodeId, branch: BranchId, v: f64) {
    // A[np, branch] = 1, A[nm, branch] = -1
    // A[branch, np] = 1, A[branch, nm] = -1
    // z[branch] = V
}
```

### Capacitor Companion Model

Using trapezoidal integration, a capacitor becomes:

```rust
// Equivalent conductance
let g_eq = 2.0 * c / dt;
// History current
let i_eq = g_eq * v_prev + i_prev;

// Stamp as resistor + current source
matrix.stamp_conductance(n1, n2, g_eq);
matrix.add_source(n1, -i_eq);
matrix.add_source(n2, i_eq);
```

### LU Decomposition

The system is solved using LU decomposition with partial pivoting:

```rust
impl MnaMatrix {
    pub fn lu_decompose(&mut self) { /* ... */ }
    pub fn lu_solve(&mut self) { /* ... */ }
}
```

This allows efficient re-solving when only the source vector changes (linear circuits).

---

## Newton-Raphson Iteration

### Algorithm

For circuits with nonlinear components (diodes, BJTs):

```rust
pub fn solve(&mut self, circuit: &Circuit, matrix: &mut MnaMatrix) -> bool {
    for iteration in 0..self.max_iterations {
        // 1. Stamp linear components
        stamp_linear_components(circuit, matrix, dt);

        // 2. Stamp nonlinear components at current operating point
        stamp_nonlinear_components(circuit, matrix, &self.x_prev);

        // 3. Solve linear system
        matrix.lu_decompose();
        matrix.lu_solve();

        // 4. Check convergence
        let residual = max_voltage_change(&matrix.x, &self.x_prev);
        if residual < self.tolerance {
            return true; // Converged
        }

        // 5. Update operating points
        self.x_prev.copy_from_slice(&matrix.x);
    }
    false // Failed to converge
}
```

### Convergence Criteria

Convergence is checked by maximum voltage change:

$$\text{residual} = \max_i |x_i^{(k+1)} - x_i^{(k)}|$$

Default tolerance: $10^{-4}$ V (configurable)

### Voltage Limiting

To prevent numerical overflow in exponential functions:

```rust
fn limit_voltage(v: f64, v_crit: f64, n_vt: f64) -> f64 {
    if v > v_crit {
        v_crit + n_vt * (1.0 + (v - v_crit) / n_vt).ln()
    } else {
        v
    }
}
```

---

## Audio Processing Pipeline

### Simulator Structure

```rust
pub struct Simulator {
    circuit: Circuit,
    matrix: MnaMatrix,
    newton: NewtonRaphson,
    sample_rate: f32,
    dt: f64,
    delays: Vec<InCircuitDelay>,
    reverbs: Vec<InCircuitReverb>,
    lfos: HashMap<String, Lfo>,
    has_modulation: bool,
}
```

### Processing Loop

```rust
impl Simulator {
    pub fn step(&mut self) -> Result<f32> {
        // 1. Advance LFOs
        for lfo in self.lfos.values_mut() {
            lfo.advance();
        }

        // 2. Update modulated resistors
        if self.has_modulation {
            self.update_modulated_components();
        }

        // 3. Process digital effects (read prev solution, compute output)
        for delay in &mut self.delays {
            delay.output_voltage = delay.effect.process(prev_input);
        }

        // 4. Clear and re-stamp matrix
        self.matrix.clear();
        stamp_linear_components(&self.circuit, &mut self.matrix, self.dt);

        // 5. Solve (with Newton-Raphson if nonlinear)
        if self.circuit.has_nonlinear {
            self.newton.solve(&self.circuit, &mut self.matrix);
        } else {
            self.matrix.lu_decompose();
            self.matrix.lu_solve();
        }

        // 6. Read output voltage
        let v_out = self.matrix.node_voltage(&self.circuit, self.circuit.output_node);
        Ok(v_out as f32)
    }
}
```

### Input Signal Injection

The input audio sample sets the V_IN source voltage:

```rust
pub fn set_input(&mut self, sample: f32) {
    // Find V_IN component and update its value
    for component in &mut self.circuit.components {
        if let Component::VoltageSource(vs) = component {
            if vs.name == "V_IN" {
                vs.ac_value = sample as f64;
            }
        }
    }
}
```

---

## Digital Effects Integration

### In-Circuit Placement

Digital effects (DELAY, REVERB) are integrated as voltage sources within the MNA matrix, not as post-processing.

**Advantages:**
- Effects can be placed anywhere in the circuit topology
- Analog and digital components interact naturally
- Feedback loops work correctly

**Trade-off:**
- 1-sample latency (similar to capacitor companion models)

### Implementation Pattern

```rust
struct InCircuitDelay {
    effect: DelayLine,
    input_node: NodeId,
    output_node: NodeId,
    branch: BranchId,
    output_voltage: f64,
}

// During step():
// 1. Read input from previous solution
let input_v = self.matrix.node_voltage(&self.circuit, delay.input_node);

// 2. Process through effect
delay.output_voltage = delay.effect.process(input_v as f32) as f64;

// 3. Stamp as voltage source driving output node
self.matrix.stamp_voltage_source(
    delay.output_node,
    NodeId::GROUND,
    delay.branch,
    delay.output_voltage
);
```

---

## LFO Modulation System

### LFO Structure

```rust
pub struct Lfo {
    pub name: String,
    pub rate: f64,        // Hz
    pub shape: LfoShape,
    pub phase: f64,       // 0.0 to 1.0
    sample_rate: f64,
}

pub enum LfoShape {
    Sine,
    Triangle,
    Sawtooth,
    Square,
}
```

### Phase Accumulation

```rust
impl Lfo {
    pub fn advance(&mut self) {
        self.phase += self.rate / self.sample_rate;
        self.phase %= 1.0;
    }

    pub fn value(&self) -> f64 {
        match self.shape {
            LfoShape::Sine => 0.5 * (1.0 + (2.0 * PI * self.phase).sin()),
            LfoShape::Triangle => {
                let t = 2.0 * self.phase;
                if t < 1.0 { t } else { 2.0 - t }
            }
            // ...
        }
    }
}
```

### Modulated Resistor Update

```rust
fn update_modulated_components(&mut self) {
    for component in &mut self.circuit.components {
        if let Component::Resistor(r) = component {
            if let Some(mod_config) = &r.modulation {
                if let Some(lfo) = self.lfos.get(&mod_config.lfo_name) {
                    let lfo_val = lfo.value();
                    r.effective_resistance = r.base_resistance
                        * (1.0 + mod_config.depth * mod_config.range * lfo_val);
                }
            }
        }
    }
}
```

---

## WASM Architecture

### Bindings Structure

```rust
#[wasm_bindgen]
pub struct WasmPedalSim {
    simulator: Simulator,
}

#[wasm_bindgen]
impl WasmPedalSim {
    #[wasm_bindgen(constructor)]
    pub fn new(circuit_dsl: &str, sample_rate: f32) -> Result<WasmPedalSim, JsValue>;

    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]);

    pub fn node_voltage(&self, node_name: &str) -> Option<f64>;
}
```

### Memory Model

- WASM linear memory holds the MNA matrix, buffers, state
- TypedArrays (Float32Array) passed directly without copying
- `process_block` writes directly to output buffer

### AudioWorklet Integration

```
┌─────────────────────────────────────────────────────────┐
│                    Main Thread                          │
│  ┌──────────────┐      ┌────────────────────────┐      │
│  │   JS/TS App  │─────►│  AudioWorkletNode      │      │
│  └──────────────┘      │  (port.postMessage)    │      │
│                        └────────────────────────┘      │
└─────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────┐
│                 AudioWorklet Thread                     │
│  ┌────────────────────────────────────────────────┐    │
│  │           AudioWorkletProcessor                │    │
│  │  ┌────────────────────────────────────────┐   │    │
│  │  │         WASM Module                    │   │    │
│  │  │  ┌─────────────────────────────────┐  │   │    │
│  │  │  │        WasmPedalSim             │  │   │    │
│  │  │  │  - Simulator                    │  │   │    │
│  │  │  │  - MNA Matrix                   │  │   │    │
│  │  │  │  - Component State              │  │   │    │
│  │  │  └─────────────────────────────────┘  │   │    │
│  │  └────────────────────────────────────────┘   │    │
│  │                                               │    │
│  │  process(inputs, outputs) {                   │    │
│  │    this.sim.process_block(input, output);     │    │
│  │  }                                            │    │
│  └────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## Performance Considerations

### Hot Path Optimization

The `step()` function is called 48,000 times per second. Critical optimizations:

1. **No heap allocations** - All buffers pre-allocated
2. **Dense matrix** - Row-major for cache efficiency
3. **Early termination** - Newton-Raphson exits on convergence
4. **Minimal branching** - Linear-only circuits skip NR

### Matrix Size

For typical pedal circuits:
- 5-20 nodes
- 5-15 components
- Matrix size: 10×10 to 30×30

Dense storage is more efficient than sparse for these sizes.

### Tolerance Trade-off

| Tolerance | Iterations | Use Case |
|-----------|------------|----------|
| 1e-6 | 5-15 | Offline rendering |
| 1e-4 | 2-5 | Real-time (default) |
| 1e-3 | 1-3 | Real-time, complex circuits |

---

## See Also

- [DSL Reference](./dsl_reference.md) - Circuit description syntax
- [Component Models](./components.md) - Component physics and math
- [WASM Integration](./wasm_integration.md) - Web application usage
