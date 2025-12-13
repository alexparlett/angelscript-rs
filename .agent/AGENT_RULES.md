# Context Engineering Rules

Based on research from Google ADK, Stanford ACE, and Manus on building reliable long-running agents.

## The Core Problem

Agents degrade over time not because they run out of memory, but because **every token competes for attention**. Stuff 100K tokens of history into the window and the model's ability to reason about what actually matters degrades.

## Four-Layer Memory Model

### Layer 1: Working Context (Cache - Expensive, Limited)
What actually gets sent to the model on each call.
- Should be as SMALL as possible while remaining sufficient
- Computed fresh for each step, not accumulated
- Contains: current task, relevant constraints, necessary context only

### Layer 2: Sessions (RAM - Larger, Bounded)
Structured event log of everything that happened.
- Full history in typed records (not raw prompts)
- Source of truth for reconstruction
- Never sent directly to model - used to compute working context

### Layer 3: Memory (Disk - Searchable)
Queryable knowledge retrieved on demand.
- Strategies that worked
- Constraints still active
- Failures to avoid
- Key entities and references

### Layer 4: Artifacts (External Storage)
Large objects stored by reference.
- Tool outputs written to files
- Code snapshots
- Documents
- Model sees pointers, not content

## Nine Scaling Principles

### 1. Context is Computed, Not Accumulated
Every LLM call gets a freshly computed view. Ask:
- What's relevant NOW?
- What instructions apply NOW?
- Which artifacts matter NOW?

❌ Wrong: Append everything to growing transcript
✅ Right: Compute minimal relevant context per step

### 2. Separate Storage from Presentation
- Sessions store everything (full fidelity)
- Working context is computed subset (optimized for decision)
- Compilation logic can change without touching storage

### 3. Scope by Default
- Default context contains nearly nothing
- Information enters through explicit retrieval
- Forces agent to decide what's worth including

### 4. Retrieval Over Pinning
- Don't pin everything permanently in context
- Treat memory as something queried on demand
- Relevance-ranked results, not accumulated history

### 5. Schema-Driven Summarization
Before compressing, define what MUST survive:
- **Causal steps**: Chain of decisions and why
- **Active constraints**: Rules still in effect
- **Failures**: What was tried and didn't work
- **Open commitments**: Promises not yet fulfilled
- **Key entities**: Names and references that must stay resolvable

❌ Wrong: Summarize "to save space" without preservation schema
✅ Right: Explicit schema guarantees critical info survives

### 6. Offload to Filesystem
- Don't feed model raw tool results at scale
- Write to disk, pass pointers
- Filesystem is unlimited context with persistence

### 7. Isolate Context with Sub-Agents
- Sub-agents exist to give different work its own window
- NOT to roleplay human teams
- Communication through structured artifacts, not shared transcripts

### 8. Design for Cache Stability
- Keep prompt prefix stable (no timestamps at start!)
- Make context append-only
- Deterministic serialization
- Cache hit = 10x cost savings

### 9. Let Context Evolve
- Static prompts freeze agents at version one
- Capture execution feedback
- Update strategies based on what worked/failed
- Agent that ran this morning informs agent running this afternoon

## Failure Modes to Avoid

1. **Append-everything trap**: Growing transcript until degradation
2. **Blind summarization**: Compressing without preservation schema
3. **Long-context delusion**: Thinking bigger windows solve the problem
4. **Observability as context**: Mixing debug logs with task instructions
5. **Tool schema bloat**: Too many overlapping tools
6. **Anthropomorphic multi-agent**: Agents roleplaying teams, sharing giant context
7. **Static configurations**: Never learning from execution
8. **Over-structured harness**: Architecture that bottlenecks model capability
9. **Cache destruction**: Unstable prefixes, non-deterministic serialization

## Working Context Assembly

For each LLM call, assemble working context by:

1. Start with stable system prefix (cached)
2. Load current task from feature_list.json
3. Retrieve relevant memory items (not all!)
4. Include only active constraints
5. Reference artifacts by path (not content)
6. Add recent session events (last N, not all)
7. Include current step instructions

Total should be minimal - justify every token.
