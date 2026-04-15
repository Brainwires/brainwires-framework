/**
 * Tests for the code_analysis module.
 *
 * Covers:
 * - Symbol extraction from TypeScript source
 * - Symbol extraction from Python source
 * - Symbol extraction from Rust source
 * - Call graph building
 * - Repo map formatting
 * - Reference kind detection
 */

import { assertEquals, assertExists } from "@std/assert";
import {
  buildCallGraph,
  definitionToStorageId,
  determineReferenceKind,
  findReferences,
  RepoMap,
} from "./mod.ts";
import type { Definition } from "./mod.ts";

// ── TypeScript symbol extraction ─────────────────────────────────────────────

Deno.test("extractSymbols — TypeScript functions", () => {
  const source = `
export function greet(name: string): string {
  return \`Hello, \${name}!\`;
}

function helper() {
  return 42;
}
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/greet.ts", content: source });

  assertEquals(defs.length, 2);

  const greet = defs.find((d) => d.symbolId.name === "greet");
  assertExists(greet, "should find greet");
  assertEquals(greet.symbolId.kind, "function");
  assertEquals(greet.visibility, "public"); // "export" keyword

  const helper = defs.find((d) => d.symbolId.name === "helper");
  assertExists(helper, "should find helper");
  assertEquals(helper.visibility, "private");
});

Deno.test("extractSymbols — TypeScript classes and interfaces", () => {
  const source = `
export class Person {
  name: string;
  constructor(name: string) {
    this.name = name;
  }
}

export interface Greeter {
  greet(name: string): string;
}

type ID = string | number;
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/types.ts", content: source });

  const person = defs.find((d) => d.symbolId.name === "Person");
  assertExists(person, "should find Person class");
  assertEquals(person.symbolId.kind, "class");

  const greeter = defs.find((d) => d.symbolId.name === "Greeter");
  assertExists(greeter, "should find Greeter interface");
  assertEquals(greeter.symbolId.kind, "interface");

  const id = defs.find((d) => d.symbolId.name === "ID");
  assertExists(id, "should find ID type alias");
  assertEquals(id.symbolId.kind, "type_alias");
});

Deno.test("extractSymbols — TypeScript enums and variables", () => {
  const source = `
export enum Color {
  Red,
  Green,
  Blue,
}

export const MAX_SIZE = 100;
let counter = 0;
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/constants.ts", content: source });

  const color = defs.find((d) => d.symbolId.name === "Color");
  assertExists(color, "should find Color enum");
  assertEquals(color.symbolId.kind, "enum");

  const maxSize = defs.find((d) => d.symbolId.name === "MAX_SIZE");
  assertExists(maxSize, "should find MAX_SIZE");
  assertEquals(maxSize.symbolId.kind, "variable");

  const counter = defs.find((d) => d.symbolId.name === "counter");
  assertExists(counter, "should find counter");
});

// ── Python symbol extraction ─────────────────────────────────────────────────

Deno.test("extractSymbols — Python functions and classes", () => {
  const source = `
# A greeting function
def greet(name):
    """Say hello."""
    print(f"Hello, {name}!")

class MyClass:
    def __init__(self, value):
        self.value = value

    def get_value(self):
        return self.value

async def fetch_data(url):
    pass
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/main.py", content: source });

  const greet = defs.find((d) => d.symbolId.name === "greet");
  assertExists(greet, "should find greet");
  assertEquals(greet.symbolId.kind, "function");

  const myClass = defs.find((d) => d.symbolId.name === "MyClass");
  assertExists(myClass, "should find MyClass");
  assertEquals(myClass.symbolId.kind, "class");

  const init = defs.find((d) => d.symbolId.name === "__init__");
  assertExists(init, "should find __init__");

  const getValue = defs.find((d) => d.symbolId.name === "get_value");
  assertExists(getValue, "should find get_value");

  const fetchData = defs.find((d) => d.symbolId.name === "fetch_data");
  assertExists(fetchData, "should find async function fetch_data");
});

// ── Rust symbol extraction ───────────────────────────────────────────────────

Deno.test("extractSymbols — Rust symbols", () => {
  const source = `
/// A greeting function
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

struct Person {
    name: String,
}

impl Person {
    fn new(name: String) -> Self {
        Self { name }
    }
}

pub enum Color {
    Red,
    Green,
    Blue,
}

trait Greetable {
    fn greet(&self) -> String;
}

mod utils {
    pub fn helper() {}
}

const MAX: usize = 100;
type Alias = Vec<String>;
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/lib.rs", content: source });

  const names = defs.map((d) => d.symbolId.name);
  const hasName = (n: string) => names.includes(n);

  assertEquals(hasName("greet"), true, "should find greet fn");
  assertEquals(hasName("Person"), true, "should find Person struct or impl");
  assertEquals(hasName("new"), true, "should find new fn");
  assertEquals(hasName("Color"), true, "should find Color enum");
  assertEquals(hasName("Greetable"), true, "should find Greetable trait");
  assertEquals(hasName("utils"), true, "should find utils mod");
  assertEquals(hasName("MAX"), true, "should find MAX const");
  assertEquals(hasName("Alias"), true, "should find Alias type");

  // Check visibility
  const greet = defs.find((d) => d.symbolId.name === "greet");
  assertExists(greet);
  assertEquals(greet.visibility, "public");
});

// ── Unsupported extension ────────────────────────────────────────────────────

Deno.test("extractSymbols — unsupported extension returns empty", () => {
  const defs = RepoMap.extractSymbols({
    filePath: "data.csv",
    content: "a,b,c\n1,2,3",
  });
  assertEquals(defs.length, 0);
});

// ── Doc comment extraction ───────────────────────────────────────────────────

Deno.test("extractSymbols — extracts doc comments", () => {
  const source = `
/// Adds two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/math.rs", content: source });
  const add = defs.find((d) => d.symbolId.name === "add");
  assertExists(add);
  assertExists(add.docComment, "should extract doc comment");
  assertEquals(add.docComment!.includes("Adds two numbers"), true);
});

// ── Call graph building ──────────────────────────────────────────────────────

Deno.test("buildCallGraph — detects calls between functions", () => {
  const libSource = `
export function greet(name: string): string {
  return formatMessage(name);
}

export function formatMessage(name: string): string {
  return \`Hello, \${name}!\`;
}

export function main() {
  greet("World");
  formatMessage("Direct");
}
`;

  const defs = RepoMap.extractSymbols({ filePath: "src/lib.ts", content: libSource });
  const files = new Map<string, string>();
  files.set("src/lib.ts", libSource);

  const graph = buildCallGraph(defs, files);

  // There should be nodes for each function
  assertEquals(graph.nodes.size, 3);

  // There should be call edges
  assertEquals(graph.edges.length > 0, true, "should have call edges");

  // main should call greet
  const mainDef = defs.find((d) => d.symbolId.name === "main");
  assertExists(mainDef);
  const mainId = definitionToStorageId(mainDef);
  const mainCallees = graph.calleesOf(mainId);
  assertEquals(mainCallees.length >= 1, true, "main should call at least one function");

  // greet should call formatMessage
  const greetDef = defs.find((d) => d.symbolId.name === "greet");
  assertExists(greetDef);
  const greetId = definitionToStorageId(greetDef);
  const greetCallees = graph.calleesOf(greetId);
  assertEquals(greetCallees.length >= 1, true, "greet should call formatMessage");
});

Deno.test("CallGraph — calleeTree returns tree structure", () => {
  const source = `
export function a() {
  b();
}

export function b() {
  c();
}

export function c() {
  return 42;
}
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/chain.ts", content: source });
  const files = new Map([["src/chain.ts", source]]);
  const graph = buildCallGraph(defs, files);

  const aDef = defs.find((d) => d.symbolId.name === "a");
  assertExists(aDef);
  const tree = graph.calleeTree(definitionToStorageId(aDef), 3);
  assertExists(tree);
  assertEquals(tree.name, "a");
  // a calls b
  assertEquals(tree.children.length >= 1, true, "a should have children");
});

Deno.test("CallGraph — callersOf returns incoming edges", () => {
  const source = `
export function target() {
  return 1;
}

export function caller1() {
  target();
}

export function caller2() {
  target();
}
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/multi.ts", content: source });
  const files = new Map([["src/multi.ts", source]]);
  const graph = buildCallGraph(defs, files);

  const targetDef = defs.find((d) => d.symbolId.name === "target");
  assertExists(targetDef);
  const callers = graph.callersOf(definitionToStorageId(targetDef));
  assertEquals(callers.length >= 2, true, "target should have at least 2 callers");
});

// ── Repo map formatting ─────────────────────────────────────────────────────

Deno.test("formatRepoMap — produces grouped tree output", () => {
  const source = `
export function greet(name: string): string {
  return name;
}

export class Person {
  name: string;
}
`;
  const defs = RepoMap.extractSymbols({ filePath: "src/main.ts", content: source });
  const output = RepoMap.formatRepoMap(defs);

  assertEquals(output.includes("src/main.ts"), true, "should include file path");
  assertEquals(output.includes("greet"), true, "should include function name");
  assertEquals(output.includes("Person"), true, "should include class name");
  assertEquals(output.includes("function"), true, "should include kind label");
});

Deno.test("formatRepoMap — sorts files alphabetically", () => {
  const defsA = RepoMap.extractSymbols({
    filePath: "b.ts",
    content: "export function beta() {}",
  });
  const defsB = RepoMap.extractSymbols({
    filePath: "a.ts",
    content: "export function alpha() {}",
  });
  const output = RepoMap.formatRepoMap([...defsA, ...defsB]);
  const aIdx = output.indexOf("a.ts");
  const bIdx = output.indexOf("b.ts");
  assertEquals(aIdx < bIdx, true, "a.ts should come before b.ts");
});

// ── Reference kind detection ─────────────────────────────────────────────────

Deno.test("determineReferenceKind — call", () => {
  assertEquals(determineReferenceKind("  greet(name)", 2, "greet"), "call");
});

Deno.test("determineReferenceKind — import", () => {
  assertEquals(
    determineReferenceKind('import { greet } from "./lib"', 10, "greet"),
    "import",
  );
});

Deno.test("determineReferenceKind — instantiation", () => {
  assertEquals(
    determineReferenceKind("const p = new Person()", 14, "Person"),
    "instantiation",
  );
});

Deno.test("determineReferenceKind — write", () => {
  assertEquals(
    determineReferenceKind("counter = counter + 1", 0, "counter"),
    "write",
  );
});

Deno.test("determineReferenceKind — read (default)", () => {
  assertEquals(
    determineReferenceKind("console.log(value)", 12, "value"),
    "read",
  );
});

// ── findReferences ───────────────────────────────────────────────────────────

Deno.test("findReferences — finds call references", () => {
  const defs = RepoMap.extractSymbols({
    filePath: "src/lib.ts",
    content: "export function greet() { return 1; }",
  });

  const symbolIndex = new Map<string, Definition[]>();
  for (const def of defs) {
    const list = symbolIndex.get(def.symbolId.name) ?? [];
    list.push(def);
    symbolIndex.set(def.symbolId.name, list);
  }

  const refs = findReferences(
    "src/main.ts",
    'const x = greet();\nconsole.log("done");',
    symbolIndex,
  );

  assertEquals(refs.length >= 1, true, "should find at least one reference");
  const callRef = refs.find((r) => r.referenceKind === "call");
  assertExists(callRef, "should find a call reference to greet");
});

Deno.test("findReferences — skips definition sites", () => {
  const defs = RepoMap.extractSymbols({
    filePath: "src/lib.ts",
    content: "export function greet() { return 1; }",
  });

  const symbolIndex = new Map<string, Definition[]>();
  for (const def of defs) {
    const list = symbolIndex.get(def.symbolId.name) ?? [];
    list.push(def);
    symbolIndex.set(def.symbolId.name, list);
  }

  // Scanning the same file where greet is defined — should not create a self-reference
  const refs = findReferences(
    "src/lib.ts",
    "export function greet() { return 1; }",
    symbolIndex,
  );

  assertEquals(refs.length, 0, "should not reference definition site");
});

Deno.test("findReferences — empty symbol index returns empty", () => {
  const refs = findReferences(
    "src/main.ts",
    "greet()",
    new Map(),
  );
  assertEquals(refs.length, 0);
});

// ── Utility: supportedExtensions / languageForExtension ──────────────────────

Deno.test("RepoMap.supportedExtensions includes expected extensions", () => {
  const exts = RepoMap.supportedExtensions();
  assertEquals(exts.includes("ts"), true);
  assertEquals(exts.includes("py"), true);
  assertEquals(exts.includes("rs"), true);
  assertEquals(exts.includes("js"), true);
});

Deno.test("RepoMap.languageForExtension returns correct language", () => {
  assertEquals(RepoMap.languageForExtension("ts"), "TypeScript");
  assertEquals(RepoMap.languageForExtension("py"), "Python");
  assertEquals(RepoMap.languageForExtension("rs"), "Rust");
  assertEquals(RepoMap.languageForExtension("js"), "JavaScript");
  assertEquals(RepoMap.languageForExtension("csv"), undefined);
});
