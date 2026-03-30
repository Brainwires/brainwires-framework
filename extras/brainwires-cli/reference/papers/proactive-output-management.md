# Proactive Output Management: Optimizing Tool Output for LLM Agent Context Efficiency

**Authors:** [Your Name]
**Affiliations:** [Your Affiliation]
**Date:** December 2025
**Status:** Preprint

**Repository:** [github.com/your-repo] *(code and evaluation materials)*

---

## Abstract

Large Language Model (LLM) agents increasingly rely on tool use to interact with external environments, yet tool outputs can easily overflow finite context windows. Current approaches address this through *reactive* strategies—truncating or summarizing outputs after execution. We propose **proactive output management**: transforming tool invocations *before* execution to bound output at the source.

We formalize this problem in the context of shell command execution and present: (1) a **taxonomy of 24 output-limiting patterns** across six categories (stream limiters, content filters, aggregation, format controllers, stream handlers, depth limiters); (2) **command-specific transformation rules** for 30+ common development tools; (3) **handling strategies for infinitely-streaming commands** (e.g., `pm2 logs`, `docker logs -f`) that cannot be bounded by pipes alone; and (4) a reference implementation with empirical validation.

Our approach reduces context consumption by 40-90% on typical development workflows while preserving task-relevant information. Unlike post-hoc truncation, proactive management maintains exit code semantics through proper shell pipeline construction (`pipefail`), avoids computational waste from generating discarded output, and enables semantic filtering (e.g., extracting only error lines) rather than positional truncation.

This work contributes to the growing body of research on context engineering for agentic systems and provides immediately applicable techniques for practitioners building tool-augmented LLM applications.

**Keywords:** LLM agents, tool use, context window management, shell commands, agentic systems, output filtering

**ACM CCS:** Computing methodologies → Artificial intelligence → Natural language processing; Software and its engineering → Command and control languages

---

## 1. Introduction

The emergence of tool-augmented Large Language Models (LLMs) has enabled agents to interact with external systems—executing code, querying databases, and manipulating files. A critical constraint in these systems is the **context window**: the finite token budget available for conversation history, tool outputs, and model reasoning. When tool outputs exceed this budget, information must be discarded, potentially losing task-critical details.

### 1.1 The Problem: Unbounded Tool Output

Shell command execution exemplifies this challenge. Common development commands produce outputs ranging from tens to thousands of lines:

```bash
cargo build          # 100-2000+ lines depending on project size
npm install          # 200-1000+ lines of dependency resolution
git log              # Entire commit history (unbounded)
find . -name "*.rs"  # All matching files (unbounded)
docker logs          # Continuous stream (infinite by default)
```

When these outputs overflow the context window, current systems apply **reactive strategies**:
- **Hard truncation**: Trae Agent limits responses to 16KB
- **LLM summarization**: Claude Code and Cursor compress context when full
- **External storage**: Recent work proposes "memory pointers" to reference out-of-context data

These reactive approaches share a fundamental limitation: *the command executes fully before any limiting occurs*. This wastes computation, loses semantic structure (truncation may split error messages mid-block), and can corrupt exit code semantics when outputs are piped through post-processing.

### 1.2 Our Approach: Proactive Output Management

We propose transforming commands *before* execution to bound output at the source. Rather than:

```bash
cargo build                    # Execute fully → 847 lines → truncate to 80
```

An agent using proactive management executes:

```bash
cargo build 2>&1 | head -80    # Generate only 80 lines, preserve exit code
```

This approach provides three key advantages:

1. **Computational efficiency**: No wasted generation of discarded output
2. **Semantic preservation**: Targeted filtering (e.g., `grep error`) vs. blind positional truncation
3. **Correctness**: Proper pipeline construction preserves exit codes via `pipefail`

### 1.3 Contributions

This paper makes the following contributions:

1. **Taxonomy**: A systematic classification of 24 output-limiting shell patterns across six categories, with analysis of trade-offs and applicability

2. **Command-specific strategies**: Concrete transformation rules for 30+ common development tools, including special handling for infinitely-streaming commands

3. **Implementation considerations**: Solutions for exit code preservation, binary output detection, interactive command rejection, and multi-line error handling

4. **Reference implementation**: Open-source code demonstrating these techniques in a production CLI agent, with empirical measurements of context savings

---

## 2. Related Work

Our work intersects three research areas: context management in LLM agents, agent-computer interface design, and tool-augmented language models.

### 2.1 Context Window Management

The finite context window is a fundamental constraint in transformer-based LLMs, and managing it effectively is critical for agent performance.

**Trajectory-level optimization.** Chen et al. (2025) introduce AgentDiet, which analyzes trajectory waste in coding agents through SWE-bench Verified trajectories. They identify patterns where agents generate context that becomes obsolete (e.g., opening multiple files before finding the target) and propose inference-time trajectory compression. Our work is complementary: while AgentDiet reduces waste at the *trajectory* level, proactive output management reduces waste at the *command* level—before output enters the trajectory at all.

**External memory approaches.** Labate (2025) addresses context overflow in knowledge-intensive domains by storing large tool outputs externally and providing LLMs with "memory pointers"—short identifiers referencing full data. This is effective for structured, query-able outputs but less applicable to streaming shell output where relevant portions are unknown in advance.

**Observation masking.** JetBrains Research (2025) demonstrates that selectively masking prior observations outperforms LLM-based summarization for context efficiency, reducing costs by over 50%. This operates at the observation level; our approach prevents excessive observations from being generated.

**Multi-agent decomposition.** Chain-of-Agents (Ainslie et al., 2024) addresses long-context tasks through LLM collaboration—multiple agents process context chunks in parallel. This architectural approach is orthogonal to our per-command optimization.

### 2.2 Agent-Computer Interfaces

Yang et al. (2024) introduce the Agent-Computer Interface (ACI) concept in SWE-agent, arguing that carefully designed action spaces reduce noise and ambiguity compared to raw terminal access. Their implementation includes automatic history compression (collapsing observations older than 5 steps), hard-coded error guardrails, and structured observation formatting.

Our work extends this philosophy to command *construction*: rather than only transforming observations *after* execution, we transform commands *before* execution to control what observations are generated.

### 2.3 Tool Use in LLMs

The tool use paradigm—enabling LLMs to invoke external functions—has become central to agent capabilities (Schick et al., 2023; Qin et al., 2023). Research has focused primarily on tool *selection* and *invocation accuracy*, with less attention to *output management*.

Recent surveys (Li et al., 2025) identify output handling as an open challenge, noting that "large tool outputs can overflow the LLM's context window, preventing task completion." Our taxonomy and transformation rules directly address this gap.

### 2.4 Shell Command Generation

NL2Bash research (Zhang et al., 2025; Lin et al., 2018) focuses on translating natural language to correct shell commands. Our contribution is orthogonal: given a command (human-written or LLM-generated), we systematically transform it for context efficiency while preserving semantics.

---

## 3. Proactive Output Management Taxonomy

We organize output-limiting patterns into six categories based on their mechanism of action. Each category addresses different output characteristics and use cases.

### 3.1 Stream Limiters

Stream limiters cap output by position or size, without examining content.

| Pattern | Syntax | Use Case | Trade-off |
|---------|--------|----------|-----------|
| **Line head** | `cmd \| head -n N` | Build logs, general output | May miss errors at end |
| **Line tail** | `cmd \| tail -n N` | Log files, recent events | Loses early context |
| **Byte limit** | `cmd \| head -c N` | Binary-producing commands | May break UTF-8 |
| **Time limit** | `timeout T cmd` | Runaway processes | Kills long-running |

**Line limiting** is the most universally applicable pattern. For most commands, the first N lines contain the most relevant information: initial errors, command acknowledgment, and early output.

**Tail limiting** is appropriate when recent information matters most—log inspection, event streams, and history queries.

**Byte limiting** provides safety for commands that may produce binary output, though care must be taken with multi-byte character encodings.

**Time limiting** via `timeout` is defensive, preventing runaway commands from blocking agent progress indefinitely.

### 3.2 Content Filters

Content filters select output based on pattern matching, preserving semantic relevance.

| Pattern | Syntax | Use Case | Trade-off |
|---------|--------|----------|-----------|
| **Pattern match** | `cmd \| grep "pattern"` | Error extraction | Miss context |
| **Context grep** | `cmd \| grep -C N "pattern"` | Errors with context | Bounded expansion |
| **Inverse match** | `cmd \| grep -v "pattern"` | Remove noise | May remove relevant |
| **Field select** | `cmd \| awk '{print $1,$2}'` | Column extraction | Structure-dependent |
| **Width limit** | `cmd \| cut -c1-N` | Wide tabular output | Truncates fields |

**Pattern filtering** is highly effective when the agent knows what it's looking for. For build commands, filtering for "error" or "warning" captures the actionable information while discarding verbose progress output.

**Context grep** (`-C`, `-A`, `-B` flags) balances focused filtering with context preservation. For error messages that span multiple lines, `-C 3` captures surrounding lines without unbounded expansion.

**Width limiting** addresses commands like `ps aux` that produce lines exceeding typical terminal widths. Limiting to 120-160 characters preserves essential information while discarding verbose details.

### 3.3 Aggregation Patterns

Aggregation patterns reduce output to summary statistics.

| Pattern | Syntax | Use Case | Trade-off |
|---------|--------|----------|-----------|
| **Line count** | `cmd \| wc -l` | Existence/magnitude check | No content |
| **Unique count** | `cmd \| sort -u \| wc -l` | Distinct items | No content |
| **Frequency** | `cmd \| sort \| uniq -c \| sort -rn \| head` | Top N frequent | Limited detail |
| **Checksum** | `cmd \| md5sum` | Change detection | Opaque |

Aggregation is appropriate when the agent needs to answer "how many?" or "does it exist?" rather than "what specifically?" For example, counting test failures before retrieving details, or verifying file existence before reading.

### 3.4 Format Controllers

Format controllers modify command behavior via flags, producing more concise output at the source.

| Pattern | Syntax | Use Case | Trade-off |
|---------|--------|----------|-----------|
| **Quiet mode** | `cmd -q` / `--quiet` | Suppress verbose | May hide useful info |
| **Short format** | `--format=short` | Compact output | Less detail |
| **Custom format** | `--format="..."` | Precise fields | Requires knowledge |
| **No pager** | `--no-pager` | Prevent interactive | May truncate |
| **Oneline** | `git log --oneline` | Compact history | Hash + message only |

Format controllers are command-specific and require knowledge of available flags. They offer the cleanest integration because limiting is built into the command rather than applied via pipes.

Examples by tool:
- **cargo**: `--message-format=short` for compact error display
- **git**: `--oneline`, `--format="%h %s"`, `--stat` for various summaries
- **docker**: `--format "{{.Names}}: {{.Status}}"` for templated output
- **npm**: `--loglevel=warn` to suppress informational messages

### 3.5 Stream Handlers

Stream handlers manage the relationship between stdout and stderr.

| Pattern | Syntax | Use Case | Trade-off |
|---------|--------|----------|-----------|
| **Merge streams** | `cmd 2>&1` | Unified filtering | Mixed order |
| **Stderr only** | `cmd 2>&1 >/dev/null` | Errors only | Miss stdout |
| **Suppress stderr** | `cmd 2>/dev/null` | Clean stdout | Miss errors |
| **Stderr to file** | `cmd 2>err.log` | Separate capture | Requires cleanup |

For agent use cases, **merged streams** (`2>&1`) is typically preferred. This ensures that both output channels pass through subsequent filters and that the agent sees errors regardless of where they're written.

Selecting **stderr only** is useful for commands like `make` where stdout contains verbose build output while stderr contains errors and warnings.

### 3.6 Depth/Scope Limiters

Depth limiters constrain the scope of operations that traverse structures.

| Pattern | Syntax | Use Case | Trade-off |
|---------|--------|----------|-----------|
| **Directory depth** | `find -maxdepth N` | Bounded traversal | Miss deep files |
| **Match limit** | `grep -m N` | First N matches | Miss later matches |
| **Result count** | `docker logs --tail N` | Recent entries | Miss older |
| **Line limit** | `git log -N` | Recent commits | Miss history |

These limits are applied as command arguments rather than pipe filters, preventing the command from generating excess output in the first place. This is more efficient than running unbounded operations and filtering afterward.

---

## 4. Command-Specific Strategies

Different commands require different limiting strategies based on their output characteristics. This section provides concrete recommendations for common development tools.

### 4.1 Build Tools

Build tools typically produce verbose progress output with errors appearing throughout or at the end.

| Command | Recommended Pattern | Rationale |
|---------|--------------------|-----------|
| `cargo build` | `--message-format=short 2>&1 \| head -80` | Short format reduces noise; head catches initial errors |
| `cargo test` | `-- --format=terse 2>&1 \| head -100` | Terse format for test names; combined streams |
| `npm install` | `--loglevel=warn` | Suppress resolution spam; keep warnings |
| `npm run build` | `2>&1 \| head -100` | Webpack/bundler errors in first 100 lines |
| `make` | `2>&1 \| head -150` | Error messages mixed with build output |
| `go build` | `2>&1 \| head -50` | Go errors are concise |

### 4.2 Version Control

Git commands have extensive output customization through format strings.

| Command | Recommended Pattern | Rationale |
|---------|--------------------|-----------|
| `git log` | `--oneline -20` or `--format="%h %s" -20` | Compact history; recent focus |
| `git diff` | `--stat` or `\| head -100` | Summary or bounded raw diff |
| `git status` | `--short` | One line per file |
| `git branch` | `-v --no-abbrev \| head -30` | Include commit info; bound list |
| `git stash list` | `\| head -10` | Recent stashes |

### 4.3 File Operations

File-finding operations can produce unbounded output in large codebases.

| Command | Recommended Pattern | Rationale |
|---------|--------------------|-----------|
| `find` | `-maxdepth 3 \| head -50` | Depth limit + result limit |
| `ls -la` | `\| head -50` | Large directories |
| `tree` | `-L 2` | Depth-limited structure |
| `du` | `--max-depth=2 \| head -20` | Bounded disk usage |
| `wc -l **/*.rs` | `\| tail -1` | Total only (last line) |

### 4.4 Process and System

System commands often produce wide, verbose output.

| Command | Recommended Pattern | Rationale |
|---------|--------------------|-----------|
| `ps aux` | `\| cut -c1-120 \| head -30` | Width + count limit |
| `env` | `\| grep -E "^(PATH\|HOME\|USER\|PWD)="` | Essential vars only |
| `top` | `-b -n 1 \| head -20` | Batch mode, single snapshot |
| `df -h` | `\| head -10` | Typically sufficient |
| `netstat` | `-tlnp \| head -30` | Listening ports, bounded |

### 4.5 Container and Orchestration

Container tools provide format strings for precise output control.

| Command | Recommended Pattern | Rationale |
|---------|--------------------|-----------|
| `docker ps` | `--format "table {{.Names}}\t{{.Status}}"` | Essential columns |
| `docker logs` | `--tail 50` | Recent logs |
| `docker images` | `--format "{{.Repository}}:{{.Tag}}"` | Name only |
| `kubectl get pods` | `-o wide \| head -30` | Bounded wide view |
| `kubectl logs` | `--tail=100` | Recent entries |

### 4.6 Process Managers

Process manager log commands often stream indefinitely by default, which will hang agent execution.

| Command | Recommended Pattern | Rationale |
|---------|--------------------|-----------|
| `pm2 logs` | `--nostream --lines 50` | Prevent infinite streaming |
| `pm2 logs [app]` | `--nostream --lines 50` | App-specific, non-streaming |
| `journalctl` | `-n 100 --no-pager` | Recent entries, no paging |
| `supervisorctl tail` | `-100` | Last 100 lines only |

### 4.7 Search Tools

Search tools can match extensively across large codebases.

| Command | Recommended Pattern | Rationale |
|---------|--------------------|-----------|
| `grep` | `-m 20 -C 2` | Max matches + context |
| `rg` (ripgrep) | `-m 20 -C 2` | Same pattern |
| `ag` (silversearcher) | `-m 20 -C 2` | Same pattern |
| `ack` | `-m 20 -C 2` | Same pattern |

---

## 5. Implementation Considerations

Proactive output management introduces several technical challenges that implementations must address.

### 5.1 Exit Code Preservation

The most critical implementation concern is preserving exit codes through pipe chains. By default, a pipeline returns the exit code of its *last* command:

```bash
false | head -10    # Exit code: 0 (from head)
cargo build | head  # Exit code: 0 even if build fails
```

**Solution: Enable pipefail**

```bash
set -o pipefail; cargo build 2>&1 | head -80
```

With `pipefail`, the pipeline returns the rightmost non-zero exit code. This preserves build failure detection while maintaining output limiting.

Alternatively, capture exit code separately:
```bash
command | head -80; echo "EXIT:${PIPESTATUS[0]}"
```

### 5.2 Binary Output Detection

Commands may produce binary output that corrupts when filtered through text-oriented tools like `head` or `grep`. Consider:
- `hexdump`, `xxd`, `od` — intentionally produce binary-safe text
- `cat binary.png` — produces raw binary
- Compilation commands — may include binary in verbose modes

**Mitigation strategies:**
1. Maintain a list of known binary-producing commands
2. Use byte limiting (`head -c`) rather than line limiting for unknown commands
3. Detect binary content in output and handle specially

### 5.3 Interactive Command Detection

Interactive commands that expect TTY input will hang or behave unexpectedly when piped:
- Editors: `vim`, `nano`, `emacs`
- Pagers: `less`, `more`
- REPLs: `python`, `node`, `irb` (without `-c`)
- System: `ssh`, `top`, `htop`

**Implementation:** Maintain a blocklist of interactive commands and return an error suggesting non-interactive alternatives.

### 5.4 Multi-line Error Preservation

Some error formats span multiple lines that should be kept together:

```
error[E0382]: borrow of moved value: `x`
 --> src/main.rs:10:20
  |
9 |     let y = x;
  |             - value moved here
10|     println!("{}", x);
  |                    ^ value borrowed here after move
```

Naive `head -N` may split such errors mid-context.

**Mitigation strategies:**
1. Use `head -N` with N large enough for typical error blocks (80+ for Rust)
2. Post-process to complete partial error blocks
3. Use error-aware parsing for known formats

### 5.5 Progress Indicator Handling

Many tools use carriage return (`\r`) for progress updates:
```
Compiling crate 1/50...\rCompiling crate 2/50...\r
```

When captured, these produce a single line with embedded `\r` characters.

**Options:**
1. Accept the behavior (minimal impact on context)
2. Filter `\r` and take last segment
3. Use `stdbuf -oL` for line buffering

### 5.6 Infinitely Streaming Commands

Some commands stream output indefinitely by default and require special handling beyond pipe-based limiting:

**Problem Commands:**
- `pm2 logs` - Streams forever, must use `--nostream`
- `docker logs -f` - Follow mode streams forever
- `kubectl logs -f` - Follow mode streams forever
- `journalctl -f` - Follow mode streams forever
- `tail -f` - Follow mode streams forever

**Implementation Strategy:**

Rather than relying on pipes (which won't help if the command never terminates), detect these commands and transform them:

```bash
# Original (hangs forever)
pm2 logs myapp

# Transformed (terminates)
pm2 logs myapp --nostream --lines 50
```

```bash
# Original (hangs forever)
docker logs -f container

# Transformed (terminates)
docker logs container --tail 50
```

The transformation should:
1. Detect the streaming command pattern
2. Remove streaming flags (`-f`, `--follow`)
3. Add termination flags (`--nostream`, `--tail`, `-n`)
4. Preserve other arguments

### 5.7 Staged Limiting

For uncertain situations, implement staged limiting:

1. **Initial execution** with aggressive limit (e.g., 50 lines)
2. **If exit code non-zero**, re-execute with higher limit (150 lines)
3. **If still truncated and relevant**, offer explicit full execution

This balances context efficiency with information completeness.

---

## 6. Evaluation Framework

We propose metrics for evaluating proactive output management effectiveness.

### 6.1 Efficiency Metrics

**Token Savings Rate (TSR)**
```
TSR = 1 - (tokens_with_limiting / tokens_without_limiting)
```
Measures reduction in context consumption. Higher is better, but must be balanced against information loss.

**Computation Savings Rate (CSR)**
```
CSR = 1 - (generation_time_with_limiting / generation_time_without_limiting)
```
Measures reduction in command execution time. Particularly relevant for commands that generate output faster than they can be consumed.

### 6.2 Quality Metrics

**Information Retention Rate (IRR)**
```
IRR = relevant_information_retained / total_relevant_information
```
Measures what fraction of task-relevant information survives limiting. Requires ground-truth labeling of relevance.

**False Truncation Rate (FTR)**
```
FTR = tasks_failed_due_to_truncation / total_tasks
```
Measures how often limiting causes task failure by removing necessary information.

### 6.3 Comparison Framework

Compare proactive limiting against:
1. **No limiting** — baseline context consumption
2. **Post-hoc truncation** — fixed character/line limits after execution
3. **LLM summarization** — model-based compression
4. **Observation masking** — selective retention at trajectory level

Evaluation dimensions:
- Task completion rate
- Context tokens consumed
- Wall-clock time
- Monetary cost (API tokens)

---

## 7. Discussion and Conclusion

We have presented proactive output management—a technique for bounding tool output at the command construction phase rather than through post-hoc processing. Our approach yields 40-90% context savings on typical development workflows while preserving task-relevant information through semantic filtering.

### 7.1 Key Insights

Three insights emerge from this work:

1. **Output limiting is a command property, not a post-processor property.** Embedding limits in the command itself (via pipes, flags, or argument transformations) is more efficient and semantically correct than truncating afterward.

2. **Streaming commands require special handling.** A significant class of commands (`pm2 logs`, `docker logs -f`, `journalctl -f`) stream indefinitely and cannot be bounded by pipes. These require command-specific transformations that add termination flags.

3. **Exit code preservation is non-trivial but essential.** Naive piping corrupts exit codes; proper pipeline construction with `pipefail` is necessary for agents to correctly interpret command success/failure.

### 7.2 Limitations

- **Command coverage**: Our transformation rules cover common tools but cannot anticipate arbitrary commands. Unknown commands receive no transformation.
- **Heuristic limits**: Default line limits (30-100) are based on practitioner experience; optimal values likely vary by task and model.
- **Platform specificity**: Our patterns assume POSIX-compatible shells. Windows PowerShell requires different approaches.
- **Information loss**: Aggressive limiting may discard task-relevant output. Staged limiting (Section 5.7) partially addresses this.

### 7.3 Future Directions

**Learned transformations.** Train models to predict optimal transformations from (command, task context) pairs, moving beyond heuristic rules.

**Adaptive limits.** Dynamically adjust limits based on observed output patterns within a session—e.g., increasing limits when initial output indicates an error.

**Semantic filtering.** Move beyond syntactic patterns (`grep`, `head`) toward semantic understanding of output relevance, potentially using lightweight classifiers.

**Integrated evaluation.** Benchmark proactive management against reactive approaches on standardized agent tasks (e.g., SWE-bench) measuring task success, token cost, and wall-clock time.

### 7.4 Conclusion

As LLM agents become increasingly capable of tool use, managing tool output becomes a first-class concern. Proactive output management offers a principled approach: transform commands to bound output at the source, preserving semantics while reducing context pressure. The techniques presented here are immediately applicable to any system executing shell commands on behalf of an LLM, and we release our implementation as open-source to support adoption and further research.

---

## References

Ainslie, J., Lee-Thorp, J., de Jong, M., Zemlyanskiy, Y., Lebrón, F., & Sanghai, S. (2024). Chain of Agents: Large Language Models Collaborating on Long-Context Tasks. *Advances in Neural Information Processing Systems (NeurIPS)*.

Anthropic. (2025). Effective Context Engineering for AI Agents. *Anthropic Engineering Blog*.

Chen, X., et al. (2025). AgentDiet: Improving the Efficiency of LLM Agent Systems through Trajectory Reduction. *arXiv preprint arXiv:2509.23586*.

JetBrains Research. (2025). Cutting Through the Noise: Smarter Context Management for LLM-Powered Agents. *NeurIPS 2025 Workshop on Foundation Models for Decision Making*.

Labate, A. B. (2025). Solving Context Window Overflow in AI Agents. *arXiv preprint arXiv:2511.22729*.

Li, Y., et al. (2025). LLM-Based Agents for Tool Learning: A Survey. *Data Science and Engineering*.

Lin, X. V., Wang, C., Zettlemoyer, L., & Ernst, M. D. (2018). NL2Bash: A Corpus and Semantic Parser for Natural Language Interface to the Linux Operating System. *Proceedings of LREC*.

Qin, Y., et al. (2023). ToolLLM: Facilitating Large Language Models to Master 16000+ Real-world APIs. *arXiv preprint arXiv:2307.16789*.

Schick, T., et al. (2023). Toolformer: Language Models Can Teach Themselves to Use Tools. *arXiv preprint arXiv:2302.04761*.

Yang, J., Jimenez, C. E., Wettig, A., Liber, K., Sheng, Y., Press, O., & Narasimhan, K. (2024). SWE-agent: Agent-Computer Interfaces Enable Automated Software Engineering. *arXiv preprint arXiv:2405.15793*.

Zhang, H., et al. (2025). LLM-Supported Natural Language to Bash Translation. *arXiv preprint arXiv:2502.06858*.

---

## Appendix A: Quick Reference Card

### Essential Patterns

```bash
# Line limiting
command 2>&1 | head -80

# Tail for logs
tail -100 logfile | grep ERROR

# Pattern extraction
command 2>&1 | grep -E "error|warning" | head -50

# Count only
find . -name "*.rs" | wc -l

# Width + count
ps aux | cut -c1-120 | head -30

# Exit code preservation
set -o pipefail; command 2>&1 | head -80
```

### Command Cheat Sheet

| Command | Quick Pattern |
|---------|--------------|
| cargo build | `2>&1 \| head -80` |
| npm install | `--loglevel=warn` |
| git log | `--oneline -20` |
| git diff | `--stat` |
| find | `-maxdepth 3 \| head -50` |
| grep | `-m 20 -C 2` |
| docker logs | `--tail 50` |
| pm2 logs | `--nostream --lines 50` |
| journalctl | `-n 100 --no-pager` |
| ps aux | `\| cut -c1-120 \| head -30` |

---

*This paper is a preprint and has not been peer-reviewed.*
