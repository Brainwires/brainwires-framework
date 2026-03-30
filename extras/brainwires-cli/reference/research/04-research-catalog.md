# Research Paper Catalog — Agentic Systems Engineering

## How to Use This Catalog

Each entry follows the format:
**Title** | Year | arXiv | Core finding | Production implication

Use this catalog to:
- Justify architectural decisions with citations
- Find papers that address a specific challenge domain
- Evaluate new techniques against established baselines
- Prepare literature reviews for technical design documents

**Priority reading list** (start here for production engineering):
ReAct → MemGPT → API-Bank → Gorilla → FrugalGPT → AutoGen → DSPy → Prompt Injection (2302.12173)

---

## Domain 1: Determinism, Observability, Execution Tracing

### Language Model Cascades
**Year:** 2022
**arXiv:** [2207.10342](https://arxiv.org/abs/2207.10342)
**Authors:** David Dohan, Winnie Xu, Aitor Lewkowycz, Jacob Austin, David Bieber, Raphael Gontier, et al.
**Core finding:** Probabilistic programs ("language model cascades") provide a unifying framework for
compositional LM calls. Defines formal structure for chaining model calls with intermediate computation.
**Production implication:** Gives conceptual grounding for execution pipelines. Each model call is a
primitive; the pipeline is a program. Enables reasoning about correctness at the pipeline level, not
just the individual call level.

---

### DSPy: Compiling Declarative Language Model Calls into Self-Improving Pipelines
**Year:** 2023
**arXiv:** [2310.03714](https://arxiv.org/abs/2310.03714)
**Authors:** Omar Khattab, Arnav Singhvi, Paridhi Maheshwari, Zhiyuan Zhang, Keshav Santhanam, et al.
**Core finding:** LM pipelines can be declared as programs and compiled to optimize prompts and
few-shot demonstrations automatically. A few lines of DSPy outperform hand-crafted expert prompt
chains by 5–46%. Prompts become compiled artifacts, not hand-tuned strings.
**Production implication:** Strongest available argument that prompts should be treated as compilable
programs, not configuration text. Aligns with Principle 4 (version prompts as artifacts). The DSPy
compiler is the practical implementation of "treat prompts as code."

---

### Debugging and Monitoring LLM-based Applications (Survey)
**Year:** 2024
**arXiv:** [2407.02929](https://arxiv.org/abs/2407.02929)
**Core finding:** Survey of observability tooling for LLM applications; identifies execution tracing,
prompt versioning, and behavioral testing as the three critical infrastructure gaps.
**Production implication:** Validates the observability challenge. Confirms that log-based debugging
is insufficient; structured execution traces with model version + temperature + prompt hash are required.

---

## Domain 2: Tool Use, Schema Adherence, Contract Enforcement

### Toolformer: Language Models Can Teach Themselves to Use Tools
**Year:** 2023
**arXiv:** [2302.04761](https://arxiv.org/abs/2302.04761)
**Authors:** Timo Schick, Jane Dwivedi-Yu, Roberto Dessì, Roberta Raileanu, Maria Lomeli, Luke Zettlemoyer, et al.
**Core finding:** Models can learn to invoke external APIs (calculator, search, calendar, translator) in
a self-supervised manner. Demonstrates that tool use is learnable from few demonstrations.
**Production implication:** Establishes that tool use capability scales with training. Does NOT establish
reliability guarantees — schema adherence remains probabilistic. Baseline paper for understanding
why tool use works at all.

---

### Gorilla: Large Language Model Connected with Massive APIs
**Year:** 2023
**arXiv:** [2305.15334](https://arxiv.org/abs/2305.15334)
**Authors:** Shishir G. Patil, Tianjun Zhang, Xin Wang, Joseph E. Gonzalez
**Core finding:** Fine-tuned LLaMA-based model surpasses GPT-4 on API call writing. Combined with
document retrieval, adapts to API version changes at test time. Substantially reduces hallucination
in API call arguments compared to prompted-only approaches.
**Production implication:** Fine-tuning improves mean performance but doesn't eliminate tail risk.
"Better" schema adherence still means probabilistic adherence. External validators remain required.
Retrieval-augmented tool documentation is a practical technique for reducing tool misuse.

---

### API-Bank: Comprehensive Benchmark for Tool-Augmented LLMs
**Year:** 2023
**arXiv:** [2304.08244](https://arxiv.org/abs/2304.08244)
**Authors:** Minghao Li, Feifan Song, Bowen Yu, Haiyang Yu, Zhoujun Li, Fei Huang, Yongbin Li
**Core finding:** First large-scale benchmark for tool-augmented LLMs covering 53 APIs. Key finding:
tool call failure rates of 12–30% depending on API complexity, even with state-of-the-art models.
Categorizes failure modes: wrong API selection, wrong parameter values, wrong call order.
**Production implication:** Quantifies the tool reliability gap. Even best models fail 1-in-8 to
1-in-3 tool calls on complex APIs. This is the empirical basis for requiring external validation:
probabilistic adherence is not production-grade.

---

### StructGPT: A General Framework for LLM Reasoning over Structured Data
**Year:** 2023
**arXiv:** [2305.09645](https://arxiv.org/abs/2305.09645)
**Core finding:** Iterative reading-then-reasoning approach significantly improves LLM performance
on structured data (tables, knowledge graphs, databases). Specialized interfaces between LLMs and
structured data are more reliable than raw text interfaces.
**Production implication:** Confirms that tool contracts (typed interfaces between LLMs and data
sources) substantially outperform unstructured text interfaces. Structured tool contracts are an
engineering requirement, not a convenience.

---

## Domain 3: Memory Architecture and Retrieval

### Generative Agents: Interactive Simulacra of Human Behavior
**Year:** 2023
**arXiv:** [2304.03442](https://arxiv.org/abs/2304.03442)
**Authors:** Joon Sung Park, Joseph C. O'Brien, Carrie J. Cai, Meredith Ringel Morris, Percy Liang, Michael S. Bernstein
**Core finding:** Introduces the memory stream architecture: complete record of experiences in natural
language + retrieval by recency + importance + relevance + reflection synthesis. Agents with memory
architectures display emergent long-term behavioral consistency.
**Production implication:** First formalization of multi-factor memory retrieval (not just similarity).
Importance scoring is a practical production technique. Reflection synthesis (higher-level summaries
of episodic memories) is essential for long-running tasks. Shows memory architecture determines
behavioral quality more than model capability.

---

### MemGPT: Towards LLMs as Operating Systems
**Year:** 2023
**arXiv:** [2310.08560](https://arxiv.org/abs/2310.08560)
**Authors:** Charles Packer, Sarah Wooders, Kevin Lin, Vivian Fang, Shishir G. Patil, Ion Stoica, Joseph E. Gonzalez
**Core finding:** Maps LLM memory management to OS memory management. Context window = RAM.
External database = disk. Paging policy = self-directed memory management. LLM explicitly calls
functions to move information between tiers.
**Production implication:** Most important memory architecture paper for production systems. The OS
analogy is directly actionable: design memory systems the way OS designers think about virtual
memory. Key insight: the agent should be the memory manager, not just the memory consumer. Directly
motivates Brainwires' `TieredMemory` architecture.

---

### RETRO: Improving Language Models by Retrieving from Trillions of Tokens
**Year:** 2022
**arXiv:** [2112.04426](https://arxiv.org/abs/2112.04426)
**Authors:** Sebastian Borgeaud, Arthur Mensch, Jordan Hoffmann, Trevor Cai, Eliza Rutherford, et al.
**Core finding:** Retrieval-augmented generation with 2 trillion token retrieval corpus matches models
with 25× more parameters. Confirms retrieval as a first-class architectural primitive.
**Production implication:** Retrieval is a substitute for model size. For production systems with
domain-specific knowledge, retrieval from curated corpora is more cost-effective than larger models.
Foundation paper for RAG-based memory systems.

---

### Self-RAG: Learning to Retrieve, Generate, and Critique Through Self-Reflection
**Year:** 2023
**arXiv:** [2310.11511](https://arxiv.org/abs/2310.11511)
**Authors:** Akari Asai, Zeqiu Wu, Yizhong Wang, Avirup Sil, Hannaneh Hajishirzi
**Core finding:** Model learns to decide when retrieval is needed (not always), critiques retrieved
passages for relevance, and generates with inline citations. Outperforms RAG + ChatGPT by 11–18%
on knowledge-intensive tasks. "Retrieve on demand" reduces noise from irrelevant retrievals.
**Production implication:** Not all queries benefit from retrieval — injecting irrelevant context
reduces quality. Selective retrieval with relevance scoring is better than always-retrieve. Directly
motivates retrieval confidence thresholds.

---

## Domain 4: Planning Stability and Control Constraints

### ReAct: Synergizing Reasoning and Acting in Language Models
**Year:** 2022
**arXiv:** [2210.03629](https://arxiv.org/abs/2210.03629)
**Authors:** Shunyu Yao, Jeffrey Zhao, Dian Yu, Nan Du, Izhak Shafran, Karthik Narasimhan, Yuan Cao
**Core finding:** Interleaving reasoning traces (think) with action steps (act) significantly outperforms
acting without reasoning and reasoning without acting. On HotpotQA: ReAct reduces hallucination and
error propagation. On interactive tasks: 34% absolute improvement over RL baselines.
**Production implication:** Foundation paper for think-act-observe loops (the basis of all current
agentic frameworks). The reason to structure agent execution as alternating reasoning + tool calls.
Directly motivates the loop structure in `TaskAgent.execute()`.

---

### Plan-and-Solve Prompting: Improving Zero-Shot Chain-of-Thought Reasoning
**Year:** 2023
**arXiv:** [2305.04091](https://arxiv.org/abs/2305.04091)
**Core finding:** Explicit "devise a plan, then solve step by step" prompting substantially improves
reasoning quality over plain chain-of-thought. Separating planning from execution at the prompt level
produces more coherent multi-step solutions.
**Production implication:** Even without infrastructure changes, separating planning from execution
in the prompt improves reliability. Validates Principle 3 (separate planning from execution) at
the prompt level as well as the infrastructure level.

---

### Reflexion: Language Agents with Verbal Reinforcement Learning
**Year:** 2023
**arXiv:** [2303.11366](https://arxiv.org/abs/2303.11366)
**Authors:** Noah Shinn, Federico Cassano, Ashwin Gopinath, Karthik Narasimhan, Shunyu Yao
**Core finding:** Agents that verbally reflect on failures and store reflections in episodic memory
improve on subsequent trials. On AlfWorld: 22% absolute improvement. On HotPotQA: 20% improvement.
On HumanEval: 11% improvement. Key caveat: works when reflection has external feedback signal to
ground it.
**Production implication:** Critical caveat: Reflexion requires external feedback to be grounded.
Self-reflection without external signals degrades in practice (model convinces itself wrong answers
are correct). Validates Anti-Pattern 9: don't trust self-reflection alone; require external
validation signals.

---

### Tree of Thoughts: Deliberate Problem Solving with Large Language Models
**Year:** 2023
**arXiv:** [2305.10601](https://arxiv.org/abs/2305.10601)
**Authors:** Shunyu Yao, Dian Yu, Jeffrey Zhao, Izhak Shafran, Thomas L. Griffiths, Yuan Cao, Karthik Narasimhan
**Core finding:** Tree search over "thoughts" (intermediate reasoning steps) with external scoring
enables deliberate problem solving. On Game of 24: GPT-4 with CoT solves 4%; ToT solves 74%.
Search depth must be bounded externally.
**Production implication:** Bounded, externally-scored search is reliably better than linear
generation for complex problems. The tree structure must be externally controlled — infinite trees
cause cost explosion. Critical insight: depth bound + external scoring = reliable improvement.
Forms the basis for MDAP's voting mechanism.

---

### Graph of Thoughts: Solving Elaborate Problems with Large Language Models
**Year:** 2023
**arXiv:** [2308.09687](https://arxiv.org/abs/2308.09687)
**Authors:** Maciej Besta, Nils Blach, Ales Kubicek, Robert Gerstenberger, Michal Podstawski, et al.
**Core finding:** Generalizes ToT to arbitrary DAG structure. Thoughts can be merged (aggregation),
refined (refinement), and scored. On sorting tasks: 62% quality improvement over ToT, 31% cost
reduction.
**Production implication:** Execution as DAG (not linear chain) is both more capable and more
efficient. DAG structure enables replay, inspection, and partial re-execution. Directly motivates
Principle 5 (build execution graphs, not logs).

---

## Domain 5: Evaluation and Behavioral Reliability

### HELM: Holistic Evaluation of Language Models
**Year:** 2022
**arXiv:** [2211.09110](https://arxiv.org/abs/2211.09110)
**Authors:** Percy Liang, Rishi Bommasani, Tony Lee, Dimitris Tsipras, Dilara Soylu, et al.
**Core finding:** Single-metric evaluation of LLMs is misleading. Multi-dimensional evaluation across
accuracy, calibration, robustness, fairness, bias, toxicity, and efficiency produces fundamentally
different rankings than single-metric benchmarks. Reliability is distributional, not binary.
**Production implication:** Agent success rates must be evaluated across multiple dimensions and
distributions, not just "did it work once?" Validates Principle 9 (test behavior, not output). The
"95% success rate" metric in Brainwires testing should be disaggregated by task type and difficulty.

---

### LLM-as-a-Judge: Assessing Reliability of Automatic LLM Evaluations
**Year:** 2023
**arXiv:** [2306.05685](https://arxiv.org/abs/2306.05685)
**Core finding:** LLM judges achieve 80%+ agreement with human judges on MT-Bench. Key failure modes:
position bias (prefers first answer), verbosity bias (prefers longer answers), self-enhancement bias
(prefers answers similar to its own outputs). Agreement degrades on highly contested questions.
**Production implication:** LLM-as-judge is a practical evaluation tool but not a ground truth.
Use for high-volume screening; reserve human evaluation for contested cases and reliability threshold
setting. Correlated failure modes: LLM judge has same biases as LLM under test.

---

### Trustworthy LLMs: A Survey and Guideline for Evaluating Alignment
**Year:** 2023
**arXiv:** [2308.05374](https://arxiv.org/abs/2308.05374)
**Core finding:** Survey covering truthfulness, calibration, robustness, fairness, safety,
and alignment evaluation. Key finding: models overstate confidence (miscalibration) and performance
degrades significantly under distribution shift.
**Production implication:** Never assume model confidence scores are calibrated. Overconfidence is
the default; external grounding is required. Performance in development ≠ performance in production
due to distribution shift.

---

## Domain 6: Multi-Agent Coordination

### AutoGen: Enabling Next-Gen LLM Applications via Multi-Agent Conversation
**Year:** 2023
**arXiv:** [2308.08155](https://arxiv.org/abs/2308.08155)
**Authors:** Qingyun Wu, Gagan Bansal, Jieyu Zhang, Yiran Wu, Beibin Li, Erkang Zhu, et al.
**Core finding:** Customizable, conversable agents can accomplish complex tasks through multi-agent
conversation. Framework supports LLM + human + tool agent combinations. Empirical demonstrations
across math, coding, QA, and operations research.
**Production implication:** AutoGen's peer-to-peer conversation model works well in demos; in
production, free conversation loops require explicit termination conditions and token budgets.
Validates the need for coordination contracts (Principle 8). The framework is most reliable when
conversation patterns are explicitly programmed.

---

### CAMEL: Communicative Agents for Mind Exploration of Large Language Model Society
**Year:** 2023
**arXiv:** [2303.17760](https://arxiv.org/abs/2303.17760)
**Core finding:** Role-playing framework where agents are assigned specific roles (AI assistant, AI
user, task specifier). Structured role assignment dramatically reduces task drift compared to
unconstrained multi-agent dialogue.
**Production implication:** Role clarity reduces multi-agent chaos. Assigning explicit roles (with
authority and scope) is more reliable than emergent role allocation. Foundation paper for role
authority hierarchy design.

---

### MetaGPT: Meta Programming for A Multi-Agent Collaborative Framework
**Year:** 2023
**arXiv:** [2308.00352](https://arxiv.org/abs/2308.00352)
**Authors:** Sirui Hong, Mingchen Zhuge, Jonathan Chen, Xiawu Zheng, Yuheng Cheng, et al.
**Core finding:** Encoding Standardized Operating Procedures (SOPs) into multi-agent prompt sequences
reduces hallucination and improves code quality over chat-based multi-agent approaches. Assembly
line paradigm with specialized roles (Product Manager, Architect, Engineer, QA) outperforms
generalist agents.
**Production implication:** SOPs are the multi-agent equivalent of deterministic orchestration.
Role specialization with structured deliverables is more reliable than flexible peer conversation.
Validates single-orchestrator pattern and role authority hierarchy.

---

## Domain 7: Security, Prompt Injection, Runtime Safety

### Compromising Real-World LLM-Integrated Applications with Indirect Prompt Injection
**Year:** 2023
**arXiv:** [2302.12173](https://arxiv.org/abs/2302.12173)
**Authors:** Kai Greshake, Sahar Abdelnabi, Shailesh Mishra, Christoph Endres, Thorsten Holz, Mario Fritz
**Core finding:** LLM-integrated applications are vulnerable to indirect prompt injection via
adversarial content in documents, web pages, emails, and databases the agent retrieves. Demonstrated
on Bing GPT-4 Chat and code completion engines. Attacker can hijack agent actions via crafted content
in external sources the agent reads.
**Production implication:** Foundation paper for agent security. Every piece of external content
an agent reads is a potential attack vector. Input sanitization before context injection and output
filtering before action execution are security requirements, not optional hardening.

---

### Prompt Injection Attack Against LLM-Integrated Applications
**Year:** 2023
**arXiv:** [2306.05499](https://arxiv.org/abs/2306.05499)
**Core finding:** HouYi black-box prompt injection framework succeeds against 31/36 tested
LLM-integrated applications. Enables extraction of system prompts, impersonation of services,
and hijacking of tool calls.
**Production implication:** Quantifies the attack surface. 86% of applications tested were
vulnerable. Defense requires: instruction hierarchy (system > user > retrieved), capability
scoping, and content sandboxing. `PermissionMode` + `PolicyEngine` in Brainwires are the
defensive implementation.

---

### Formalizing and Benchmarking Prompt Injection Attacks and Defenses
**Year:** 2023
**arXiv:** [2310.12815](https://arxiv.org/abs/2310.12815)
**Core finding:** Systematic evaluation of 5 attack methods and 10 defenses across 10 LLMs and
7 tasks. Finding: all defenses reduce attack success rates but none eliminates them. Current
defenses are speed bumps, not walls.
**Production implication:** Defense-in-depth is required. No single defense is sufficient.
Layer: input sanitization + instruction hierarchy + capability scoping + output filtering.
Accept residual risk; focus on blast radius reduction.

---

## Domain 8: Cost, Latency, Systems Optimization

### FrugalGPT: How to Use Large Language Models While Reducing Cost and Improving Performance
**Year:** 2023
**arXiv:** [2305.05176](https://arxiv.org/abs/2305.05176)
**Authors:** Lingjiao Chen, Matei Zaharia, James Zou
**Core finding:** LLM cascades (route queries to cheapest model that can handle them) achieve
98% cost reduction vs. GPT-4 alone while matching or exceeding GPT-4 accuracy. Three strategies:
prompt adaptation (reduce token count), LLM approximation (use cheaper model), LLM cascade
(try cheap model first, escalate only if needed).
**Production implication:** Model routing is a first-class cost optimization technique. Not all
tasks require the most capable model. Routing by task complexity (estimated at planning time)
dramatically reduces cost. Validates Principle 7 (enforce cost budgets at runtime).

---

### Speculative Decoding: Exploiting Speculative Execution for Accelerating Sequential Generation
**Year:** 2022
**arXiv:** [2211.17192](https://arxiv.org/abs/2211.17192)
**Core finding:** Small draft model generates candidate tokens; large target model verifies in
parallel. Achieves 2–3× speedup with identical output quality to the target model alone.
**Production implication:** For local model deployments, speculative decoding is the most practical
latency reduction technique. Relevant for Brainwires' local inference path.

---

### Efficient Prompting Methods for LLMs: A Survey
**Year:** 2023
**arXiv:** [2310.01382](https://arxiv.org/abs/2310.01382)
**Core finding:** Survey of techniques for reducing prompt token count while preserving quality:
compression, summarization, selective retrieval, and few-shot example pruning. Token reduction of
30–70% achievable with <5% quality loss on standard benchmarks.
**Production implication:** Token budgets are engineering levers, not hard limits. Systematic prompt
compression is a practical technique for staying within cost budgets.

---

## Domain 9: Structured Outputs and Constrained Decoding

### Efficient Guided Generation for Large Language Models (Outlines)
**Year:** 2023
**arXiv:** [2307.09702](https://arxiv.org/abs/2307.09702)
**Authors:** Brandon T. Willard, Rémi Louf
**Core finding:** Reformulates generation as transitions between states of a finite-state machine.
Enables grammar-constrained (regex, CFG, JSON schema) generation with minimal overhead. Guarantees
structural validity of generated output.
**Production implication:** Closest existing approach to a hard guarantee on tool call structure.
Moving schema validation INTO the decoding process eliminates a class of retry loops. Does not
guarantee semantic correctness, only structural. Directly relevant to Challenge 2 (tool use).

---

### Guidance: A Guidance Language for Controlling Language Models
**Year:** 2023
**GitHub:** [microsoft/guidance](https://github.com/microsoft/guidance)
**Core finding:** Programmatic interleaving of generation and constraints. Prompts become programs
with conditional generation, loops, and token-level constraints. Structural output guarantees.
**Production implication:** Treats prompts as programs — the most principled approach to prompt
design. Enables prompt compilation (DSPy-style) and structural guarantees simultaneously.

---

## Domain 10: Formalizing LLMs as Policy Engines

### A Survey on Large Language Model based Autonomous Agents
**Year:** 2023
**arXiv:** [2308.11432](https://arxiv.org/abs/2308.11432)
**Core finding:** Survey of 200+ LLM agent papers. Identifies four key components: profiling (what
the agent is), memory (what it knows), planning (what it decides), action (what it does). Taxonomizes
failure modes across components.
**Production implication:** Useful taxonomy for diagnosing production failures. A failure is either
a planning failure, a memory failure, a tool execution failure, or a profile/identity drift failure.
Clarifies where in the stack to look when an agent fails.

---

### On the Reliability of Language Model Agents
**Year:** 2024
**arXiv:** [2407.01051](https://arxiv.org/abs/2407.01051)
**Core finding:** Systematic study of agent reliability failure modes: task misinterpretation,
hallucinated tool availability, error accumulation, and premature termination. Key finding: multi-step
reliability degrades multiplicatively — a 95% per-step success rate produces 60% success at 10 steps.
**Production implication:** Establishes the mathematical case for validation gates. At 95% per-step
reliability: P(10 steps correct) = 0.95^10 = 0.60. Every validation gate resets the accumulated
error probability. Step budgets and validation frequency are reliability engineering choices.

---

## Priority Reading List

For production engineering teams building agentic systems, read in this order:

1. **ReAct** (2210.03629) — foundation of all agentic loop design
2. **MemGPT** (2310.08560) — memory architecture as OS design problem
3. **API-Bank** (2304.08244) — empirical grounding for tool reliability gap
4. **DSPy** (2310.03714) — prompts as compilable programs
5. **FrugalGPT** (2305.05176) — cost control architecture
6. **Indirect Prompt Injection** (2302.12173) — security threat model
7. **AutoGen** (2308.08155) — multi-agent coordination patterns
8. **Tree of Thoughts** (2305.10601) — bounded search for planning reliability
9. **Outlines** (2307.09702) — structural output guarantees
10. **On the Reliability of Agents** (2407.01051) — multiplicative reliability degradation
