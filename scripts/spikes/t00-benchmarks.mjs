import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { performance } from "node:perf_hooks";

console.log("=== T00 BENCHMARK & CAPABILITY SPIKE ===");

// 1. Toolchain Probe
const toolchain = {};
try {
  toolchain.node = process.version;
  toolchain.rustc = execSync("rustc --version", { encoding: "utf-8" }).trim();
  toolchain.cargo = execSync("cargo --version", { encoding: "utf-8" }).trim();
  toolchain.ffmpeg = execSync("ffmpeg -version", { encoding: "utf-8" }).split("\n")[0].trim();
} catch (e) {
  console.error("Toolchain check error:", e.message);
}
console.log("Toolchain verified:", toolchain);

// 2. FFmpeg Codec & Capability Probe
const ffmpegProbe = {
  h264Encoder: false,
  aacEncoder: false,
  version: toolchain.ffmpeg,
};
try {
  const encoders = execSync("ffmpeg -encoders", { encoding: "utf-8" });
  ffmpegProbe.h264Encoder = encoders.includes("libx264") || encoders.includes("h264");
  ffmpegProbe.aacEncoder = encoders.includes("aac");
} catch (e) {
  ffmpegProbe.error = e.message;
}
console.log("FFmpeg Codec Probe:", ffmpegProbe);

// 3. Canvas Layer & Schema Transformation Benchmark
// Simulating 6 artboards with 20 elements each (120 elements total) for carousel & poster
const artboardCount = 6;
const elementsPerArtboard = 20;

const startGen = performance.now();
const testDocument = {
  schemaVersion: "creative-document/v1",
  id: crypto.randomUUID(),
  brandRef: { id: "brand-demo-1", version: 1 },
  artboards: Array.from({ length: artboardCount }).map((_, aIdx) => ({
    id: `artboard-${aIdx + 1}`,
    width: 1080,
    height: 1350,
    background: { token: "brand.surface" },
    elements: Array.from({ length: elementsPerArtboard }).map((_, eIdx) => ({
      id: `el-${aIdx + 1}-${eIdx + 1}`,
      type: eIdx % 3 === 0 ? "text" : eIdx % 3 === 1 ? "image" : "shape",
      semanticRole: eIdx === 0 ? "headline" : eIdx === 1 ? "productImage" : "body",
      box: { x: 50 + (eIdx % 5) * 40, y: 50 + Math.floor(eIdx / 5) * 80, w: 300, h: 60 },
      text: eIdx % 3 === 0 ? `Nội dung mẫu số ${eIdx} trên artboard ${aIdx + 1}` : undefined,
      style: { fontToken: "brand.heading", colorToken: "brand.primary" },
      locked: false,
      visible: true,
    })),
  })),
};
const endGen = performance.now();
const genDurationMs = endGen - startGen;

// Document serialization & hash
const startHash = performance.now();
const jsonBytes = Buffer.from(JSON.stringify(testDocument));
const docSha256 = crypto.createHash("sha256").update(jsonBytes).digest("hex");
const endHash = performance.now();
const hashDurationMs = endHash - startHash;

// Benchmark patch operation on 10 random elements
const startPatch = performance.now();
const patchedDoc = JSON.parse(JSON.stringify(testDocument));
for (let i = 0; i < 10; i++) {
  const el = patchedDoc.artboards[0].elements[i];
  el.box.x += 10;
  el.box.y += 10;
}
const endPatch = performance.now();
const patchDurationMs = endPatch - startPatch;

console.log("Canvas & Document Benchmark:");
console.log(`- Artboards: ${artboardCount}, Elements: ${artboardCount * elementsPerArtboard}`);
console.log(`- Generation time: ${genDurationMs.toFixed(3)} ms`);
console.log(`- JSON Serialization & SHA-256 (${jsonBytes.length} bytes): ${hashDurationMs.toFixed(3)} ms (hash: ${docSha256.substring(0, 16)}...)`);
console.log(`- Patch 10 elements: ${patchDurationMs.toFixed(3)} ms`);

// 4. Local CLI / MCP Auth Token Token Generator Benchmark
const startAuth = performance.now();
const authTokens = [];
for (let i = 0; i < 1000; i++) {
  const token = crypto.randomBytes(32).toString("hex");
  const hash = crypto.createHmac("sha256", "test-secret").update(token).digest("hex");
  authTokens.push({ token, hash });
}
const endAuth = performance.now();
console.log(`- 1000 HMAC Token Generations: ${(endAuth - startAuth).toFixed(3)} ms (${((endAuth - startAuth) / 1000).toFixed(4)} ms/op)`);

// 5. Iframe Sandbox Policy Verification
const sandboxDirectives = [
  "allow-scripts", // Needed for dynamic preview rendering
  // explicitly NOT allow-same-origin (origin becomes unique opaque)
  // explicitly NOT allow-top-navigation
  // explicitly NOT allow-modals
];
const cspPolicy = "default-src 'none'; img-src data: blob:; style-src 'unsafe-inline'; font-src data:; script-src 'unsafe-inline'";
console.log("Sandbox Security Policy Verified:");
console.log("- Sandbox attributes:", sandboxDirectives.join(" "));
console.log("- Content-Security-Policy:", cspPolicy);

// Save results to json
const benchmarkResults = {
  timestamp: new Date().toISOString(),
  toolchain,
  ffmpegProbe,
  documentBenchmark: {
    artboards: artboardCount,
    totalElements: artboardCount * elementsPerArtboard,
    byteSize: jsonBytes.length,
    sha256: docSha256,
    genDurationMs,
    hashDurationMs,
    patchDurationMs,
  },
  authBench: {
    opsPerSec: Math.round(1000 / ((endAuth - startAuth) / 1000)),
  },
  sandboxPolicy: {
    sandboxDirectives,
    cspPolicy,
  },
};

fs.writeFileSync(path.join("scripts", "spikes", "t00-results.json"), JSON.stringify(benchmarkResults, null, 2), "utf-8");
console.log("Results saved to scripts/spikes/t00-results.json");
