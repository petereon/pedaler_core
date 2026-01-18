# WASM Integration Guide for Pedaler Core

This document provides comprehensive instructions for integrating the Pedaler Core circuit simulator into web applications via WebAssembly. It covers TypeScript integration patterns with Web Audio API.

## Table of Contents

1. [Overview](#overview)
2. [Building the WASM Package](#building-the-wasm-package)
3. [Package Contents](#package-contents)
4. [API Reference](#api-reference)
5. [TypeScript Integration](#typescript-integration)
6. [AudioWorklet Patterns](#audioworklet-patterns)
7. [Error Handling](#error-handling)
8. [Performance Considerations](#performance-considerations)
9. [Troubleshooting](#troubleshooting)

---

## Overview

Pedaler Core compiles to WebAssembly for browser-based real-time audio processing. The WASM module provides:

- **Circuit simulation** via Modified Nodal Analysis (MNA)
- **Newton-Raphson iteration** for nonlinear components (diodes, transistors)
- **Sample-by-sample processing** compatible with Web Audio AudioWorklet
- **Configurable precision/performance tradeoff** via tolerance settings

### Key Characteristics

| Property | Value |
|----------|-------|
| WASM Binary Size | ~196KB (optimized) |
| Default Sample Rate | 48000 Hz |
| Default NR Tolerance | 1e-4 (volts) |
| Default Max Iterations | 50 |
| Audio Format | Mono, 32-bit float |

---

## Building the WASM Package

### Prerequisites

```bash
# Install wasm-pack
cargo install wasm-pack
# or
cargo binstall wasm-pack
```

### Build Commands

**For web (ES modules):**
```bash
wasm-pack build --target web --features wasm --no-default-features
```

**For bundlers (webpack, vite, etc.):**
```bash
wasm-pack build --target bundler --features wasm --no-default-features
```

**For Node.js:**
```bash
wasm-pack build --target nodejs --features wasm --no-default-features
```

### Build Output

The build produces files in the `pkg/` directory:

```
pkg/
├── package.json           # npm package manifest
├── pedaler_core.js        # JavaScript glue code (ES module)
├── pedaler_core.d.ts      # TypeScript type definitions
├── pedaler_core_bg.wasm   # WebAssembly binary
└── pedaler_core_bg.wasm.d.ts  # WASM memory types
```

---

## Package Contents

### Exported Types

```typescript
// Main simulator class
export class WasmPedalSim {
  constructor(circuit_dsl: string, sample_rate: number);
  static with_config(
    circuit_dsl: string,
    sample_rate: number,
    max_iterations: number,
    tolerance: number
  ): WasmPedalSim;

  process_block(input: Float32Array, output: Float32Array): void;
  process_block_alloc(input: Float32Array): Float32Array;
  node_voltage(node_name: string): number | undefined;

  readonly sample_rate: number;
  free(): void;
}

// Utility functions
export function version(): string;
export function default_sample_rate(): number;
export function init_panic_hook(): void;

// Initialization
export default function init(module_or_path?: InitInput): Promise<InitOutput>;
export function initSync(module: SyncInitInput): InitOutput;
```

### Circuit DSL Format

The simulator accepts circuits in Pedaler DSL format:

```
# Comments start with #
.input <node_name>      # Declare input node
.output <node_name>     # Declare output node

# Components: <type> <node1> <node2> <value> [model]
R1 in mid 10k           # Resistor: 10kΩ
C1 mid out 100n         # Capacitor: 100nF
D1 mid 0 1N4148         # Diode with model

# Voltage source (required for input signal injection)
V_IN in 0 DC 0          # DC source at input, value set by simulator

# Models
.model 1N4148 D IS=2.52e-9 N=1.752 VT=0.026
```

**Important:** The circuit must have:
- Exactly one `.input` directive
- Exactly one `.output` directive
- A voltage source `V_IN` at the input node for signal injection

---

## API Reference

### `WasmPedalSim` Constructor

```typescript
new WasmPedalSim(circuit_dsl: string, sample_rate: number)
```

Creates a simulator with default Newton-Raphson settings (50 iterations, 1e-4 tolerance).

**Parameters:**
- `circuit_dsl`: Circuit description in Pedaler DSL format
- `sample_rate`: Audio sample rate in Hz (typically 44100 or 48000)

**Throws:** Error if circuit DSL is invalid or circuit validation fails.

### `WasmPedalSim.with_config()` Static Method

```typescript
WasmPedalSim.with_config(
  circuit_dsl: string,
  sample_rate: number,
  max_iterations: number,
  tolerance: number
): WasmPedalSim
```

Creates a simulator with custom Newton-Raphson configuration.

**Parameters:**
- `circuit_dsl`: Circuit description
- `sample_rate`: Audio sample rate in Hz
- `max_iterations`: Maximum Newton-Raphson iterations per sample
- `tolerance`: Convergence tolerance in volts

**Tolerance Guidelines:**
| Tolerance | Use Case | Performance |
|-----------|----------|-------------|
| 1e-6 | High precision, offline rendering | Slowest |
| 1e-4 | **Default**, good for real-time | Balanced |
| 1e-3 | Fast, acceptable for most audio | Fastest |

### `process_block()` Method

```typescript
process_block(input: Float32Array, output: Float32Array): void
```

Processes audio samples in-place. This is the preferred method for AudioWorklet integration.

**Parameters:**
- `input`: Input audio samples (mono, -1.0 to 1.0 range)
- `output`: Output buffer to write results into

**Note:** The method processes `min(input.length, output.length)` samples.

### `process_block_alloc()` Method

```typescript
process_block_alloc(input: Float32Array): Float32Array
```

Processes audio and returns a new array with results. Simpler API but allocates memory.

**Parameters:**
- `input`: Input audio samples

**Returns:** New Float32Array with processed samples.

### `node_voltage()` Method

```typescript
node_voltage(node_name: string): number | undefined
```

Returns the current voltage at a named circuit node. Useful for debugging or visualization.

### `sample_rate` Property

```typescript
readonly sample_rate: number
```

Returns the sample rate the simulator was configured with.

### `free()` Method

```typescript
free(): void
```

Explicitly frees WASM memory. Called automatically when using `Symbol.dispose`.

---

## TypeScript Integration

### Project Setup

**1. Install dependencies:**

```bash
npm install pedaler_core  # If published to npm
# or copy pkg/ directory into your project
```

**2. Configure TypeScript (tsconfig.json):**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2020", "DOM"],
    "types": ["@types/audioworklet"]
  }
}
```

### Basic Usage Example

```typescript
import init, { WasmPedalSim, version } from 'pedaler_core';

async function createSimulator(): Promise<WasmPedalSim> {
  // Initialize WASM module (required once)
  await init();

  console.log(`Pedaler Core v${version()}`);

  const circuit = `
    .input in
    .output out

    # Simple voltage divider (attenuator)
    V_IN in 0 DC 0
    R1 in out 10k
    R2 out 0 10k
  `;

  const sampleRate = 48000;
  return new WasmPedalSim(circuit, sampleRate);
}

// Process a buffer
async function processAudio(input: Float32Array): Promise<Float32Array> {
  const sim = await createSimulator();
  const output = sim.process_block_alloc(input);
  sim.free(); // Clean up
  return output;
}
```

### Complete Web Audio Integration

```typescript
// src/audio/PedalerEngine.ts

import init, { WasmPedalSim } from 'pedaler_core';

export interface PedalerEngineOptions {
  circuit: string;
  sampleRate?: number;
  maxIterations?: number;
  tolerance?: number;
}

export class PedalerEngine {
  private audioContext: AudioContext | null = null;
  private workletNode: AudioWorkletNode | null = null;
  private initialized = false;

  async initialize(): Promise<void> {
    if (this.initialized) return;
    await init();
    this.initialized = true;
  }

  async connect(
    options: PedalerEngineOptions,
    source: AudioNode,
    destination: AudioNode
  ): Promise<void> {
    await this.initialize();

    const ctx = source.context as AudioContext;
    this.audioContext = ctx;

    // Register the worklet processor
    const workletUrl = new URL('./pedaler-worklet.js', import.meta.url);
    await ctx.audioWorklet.addModule(workletUrl);

    // Create worklet node
    this.workletNode = new AudioWorkletNode(ctx, 'pedaler-processor', {
      numberOfInputs: 1,
      numberOfOutputs: 1,
      outputChannelCount: [1],
      processorOptions: {
        circuit: options.circuit,
        sampleRate: options.sampleRate ?? ctx.sampleRate,
        maxIterations: options.maxIterations ?? 50,
        tolerance: options.tolerance ?? 1e-4,
      },
    });

    // Connect audio graph
    source.connect(this.workletNode);
    this.workletNode.connect(destination);
  }

  disconnect(): void {
    if (this.workletNode) {
      this.workletNode.disconnect();
      this.workletNode = null;
    }
  }

  // Send new circuit to worklet
  updateCircuit(circuit: string): void {
    this.workletNode?.port.postMessage({
      type: 'updateCircuit',
      circuit,
    });
  }
}
```

### AudioWorklet Processor (TypeScript)

```typescript
// src/audio/pedaler-worklet.ts
// NOTE: This file runs in AudioWorkletGlobalScope, not main thread

import init, { WasmPedalSim } from 'pedaler_core';

interface ProcessorOptions {
  circuit: string;
  sampleRate: number;
  maxIterations: number;
  tolerance: number;
}

class PedalerProcessor extends AudioWorkletProcessor {
  private sim: WasmPedalSim | null = null;
  private ready = false;

  constructor(options: AudioWorkletNodeOptions) {
    super();

    const opts = options.processorOptions as ProcessorOptions;

    // Initialize WASM and create simulator
    this.initializeAsync(opts);

    // Handle messages from main thread
    this.port.onmessage = (event) => {
      if (event.data.type === 'updateCircuit') {
        this.updateCircuit(event.data.circuit, opts);
      }
    };
  }

  private async initializeAsync(opts: ProcessorOptions): Promise<void> {
    try {
      await init();
      this.sim = WasmPedalSim.with_config(
        opts.circuit,
        opts.sampleRate,
        opts.maxIterations,
        opts.tolerance
      );
      this.ready = true;
      this.port.postMessage({ type: 'ready' });
    } catch (error) {
      this.port.postMessage({ type: 'error', error: String(error) });
    }
  }

  private updateCircuit(circuit: string, opts: ProcessorOptions): void {
    try {
      const newSim = WasmPedalSim.with_config(
        circuit,
        opts.sampleRate,
        opts.maxIterations,
        opts.tolerance
      );
      this.sim?.free();
      this.sim = newSim;
      this.port.postMessage({ type: 'circuitUpdated' });
    } catch (error) {
      this.port.postMessage({ type: 'error', error: String(error) });
    }
  }

  process(
    inputs: Float32Array[][],
    outputs: Float32Array[][],
    _parameters: Record<string, Float32Array>
  ): boolean {
    if (!this.ready || !this.sim) {
      return true; // Keep processor alive while initializing
    }

    const input = inputs[0]?.[0];
    const output = outputs[0]?.[0];

    if (input && output) {
      this.sim.process_block(input, output);
    }

    return true; // Keep processor running
  }
}

registerProcessor('pedaler-processor', PedalerProcessor);
```

### Vite Configuration

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  worker: {
    format: 'es',
    plugins: () => [wasm(), topLevelAwait()],
  },
  optimizeDeps: {
    exclude: ['pedaler_core'],
  },
});
```

### React Hook Example

```typescript
// src/hooks/usePedaler.ts
import { useEffect, useRef, useState, useCallback } from 'react';
import { PedalerEngine } from '../audio/PedalerEngine';

interface UsePedalerOptions {
  circuit: string;
  enabled?: boolean;
}

export function usePedaler({ circuit, enabled = true }: UsePedalerOptions) {
  const engineRef = useRef<PedalerEngine | null>(null);
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    engineRef.current = new PedalerEngine();
    return () => {
      engineRef.current?.disconnect();
    };
  }, []);

  const connect = useCallback(
    async (source: AudioNode, destination: AudioNode) => {
      if (!engineRef.current) return;

      try {
        await engineRef.current.connect({ circuit }, source, destination);
        setIsReady(true);
        setError(null);
      } catch (e) {
        setError(String(e));
      }
    },
    [circuit]
  );

  const disconnect = useCallback(() => {
    engineRef.current?.disconnect();
    setIsReady(false);
  }, []);

  // Update circuit when it changes
  useEffect(() => {
    if (isReady && enabled) {
      engineRef.current?.updateCircuit(circuit);
    }
  }, [circuit, isReady, enabled]);

  return { connect, disconnect, isReady, error };
}
```

---

## AudioWorklet Patterns

### Worklet File Loading

The AudioWorklet processor must be loaded as a separate file. Depending on your bundler:

**Vite:**
```typescript
const workletUrl = new URL('./pedaler-worklet.ts', import.meta.url);
await ctx.audioWorklet.addModule(workletUrl);
```

**Webpack:**
```typescript
await ctx.audioWorklet.addModule(new URL(
  './pedaler-worklet.ts',
  import.meta.url
));
```

**Manual (no bundler):**
```html
<script type="module">
  // pedaler-worklet.js must be served separately
  await audioContext.audioWorklet.addModule('/js/pedaler-worklet.js');
</script>
```

### WASM Loading in Worklet

AudioWorklet runs in a separate thread. You must load WASM inside the worklet:

```typescript
// pedaler-worklet.ts
import init, { WasmPedalSim } from 'pedaler_core';

class PedalerProcessor extends AudioWorkletProcessor {
  private sim: WasmPedalSim | null = null;

  constructor(options: AudioWorkletNodeOptions) {
    super();

    // WASM init is async - must handle this
    this.initWasm(options.processorOptions);
  }

  private async initWasm(opts: any): Promise<void> {
    // Load WASM - path relative to worklet file location
    await init();
    this.sim = new WasmPedalSim(opts.circuit, opts.sampleRate);
  }

  process(inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    if (!this.sim) return true; // Still loading

    const input = inputs[0]?.[0];
    const output = outputs[0]?.[0];

    if (input && output) {
      this.sim.process_block(input, output);
    }

    return true;
  }
}

registerProcessor('pedaler-processor', PedalerProcessor);
```

### Buffer Size Considerations

AudioWorklet processes in fixed 128-sample blocks. The simulator processes these efficiently:

```typescript
// Each process() call receives exactly 128 samples
// at 48kHz, this is ~2.67ms of audio
process(inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
  // inputs[0][0].length === 128 (always)
  this.sim?.process_block(inputs[0][0], outputs[0][0]);
  return true;
}
```

---

## Error Handling

### Circuit Parsing Errors

```typescript
try {
  const sim = new WasmPedalSim(invalidCircuit, 48000);
} catch (error) {
  // Error messages include line/column info
  // e.g., "Parse error at line 5, column 12: unexpected token"
  console.error('Circuit error:', error);
}
```

### Validation Errors

The simulator validates circuits for:
- Missing `.input` or `.output` directives
- Floating nodes (not connected to ground path)
- Missing voltage source at input
- Unknown component models

### Runtime Errors

If Newton-Raphson fails to converge, `step()` returns 0.0 (silence) rather than crashing. This prevents audio glitches but may indicate:
- Circuit has no solution (unrealistic component values)
- Tolerance too tight for the circuit
- Max iterations too low

---

## Performance Considerations

### Optimization Settings

For production builds, use the release-wasm profile:

```bash
wasm-pack build --target web --features wasm --no-default-features --release
```

The `Cargo.toml` includes an optimized profile:

```toml
[profile.release-wasm]
inherits = "release"
opt-level = "s"  # Optimize for size
lto = true
```

### Tolerance vs Performance

| Tolerance | Typical Iterations | Real-time Safe |
|-----------|-------------------|----------------|
| 1e-6 | 5-15 | Maybe |
| 1e-4 | 2-5 | Yes |
| 1e-3 | 1-3 | Yes |

For complex circuits with many diodes/transistors, use 1e-3 tolerance.

### Memory Management

- Call `sim.free()` when done to release WASM memory
- Avoid creating new simulators during audio processing
- Reuse the same simulator instance

### Avoiding Allocations

Use `process_block()` instead of `process_block_alloc()` in AudioWorklet:

```typescript
// Good - no allocation
sim.process_block(input, output);

// Avoid in real-time - allocates new array
const result = sim.process_block_alloc(input);
```

---

## Troubleshooting

### "Module not found" in AudioWorklet

AudioWorklet runs in a separate scope. Ensure:
1. WASM file is accessible from worklet's location
2. Use correct relative paths
3. Bundler is configured to handle worklet imports

### WASM Loading Fails

Check browser console for CORS errors. WASM must be served with:
```
Content-Type: application/wasm
```

### Audio Glitches / Dropouts

1. Increase tolerance: `WasmPedalSim.with_config(circuit, sr, 50, 1e-3)`
2. Reduce circuit complexity
3. Check CPU usage in browser dev tools
4. Ensure no GC pauses (avoid allocations in audio callback)

### Circuit Not Producing Output

1. Verify `.input` and `.output` directives exist
2. Check `V_IN` voltage source is at input node
3. Use `node_voltage()` to debug intermediate values
4. Verify ground connections (node `0`)

---

## Example Circuits

### Simple Attenuator (Voltage Divider)

```
.input in
.output out

V_IN in 0 DC 0
R1 in out 10k
R2 out 0 10k
```

### RC Low-Pass Filter

```
.input in
.output out

V_IN in 0 DC 0
R1 in out 1k
C1 out 0 100n
```

### Diode Clipper (Soft Distortion)

```
.input in
.output out

V_IN in 0 DC 0
R1 in clip 4.7k
D1 clip 0 1N4148
D2 0 clip 1N4148
C1 clip out 100n
R2 out 0 10k

.model 1N4148 D IS=2.52e-9 N=1.752 VT=0.026
```

### Tube Screamer Style Overdrive

```
.input in
.output out

V_IN in 0 DC 0

# Input buffer
R1 in buf 10k
C1 buf 0 47n

# Clipping stage
R2 buf clip 4.7k
D1 clip 0 1N4148
D2 0 clip 1N4148

# Tone control
R3 clip tone 1k
C2 tone 0 220n

# Output
R4 tone out 10k
R5 out 0 10k

.model 1N4148 D IS=2.52e-9 N=1.752 VT=0.026
```

---

## Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial WASM support |

---

## Related Documentation

- [Pedaler DSL Reference](./dsl_reference.md) - Complete DSL syntax
- [Component Models](./components.md) - Supported components
- [Architecture Overview](./architecture.md) - Internal design

---

## License

MIT License - See LICENSE file in repository root.
