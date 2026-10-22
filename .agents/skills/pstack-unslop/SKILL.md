---
name: pstack-unslop
description: Edit authored XDRemux prose to remove vague claims, AI phrasing, unnecessary jargon, and repetitive framing while preserving meaning, evidence, identifiers, and intended tone. Use for explanations, documentation, review reports, and requested writing cleanup.
---

# Plain, evidence-backed prose

State the outcome early. Use familiar words, active verbs, and connected sentences. Preserve the intended meaning, source attribution, evidence limits, and the reader's level of technical detail. Respect the current repository's English-canonical and matching-Chinese documentation policy.

Cut filler, generic praise, chatbot closers, decorative emoji, vague attribution, and inflated claims. Name the source or remove an attribution such as "experts believe". Replace abstract claims with the mechanism, observed behavior, or measured number that supports them. Do not turn an inference into a fact while shortening it.

Prefer "use", "help", "is", and "has" to elaborate synonyms. Avoid forced groups of three, synonym cycling, false ranges, "not just X but Y" framing, mannered metaphors, and stock conclusions. Keep one main idea per sentence and enough articles and verbs to read naturally. Do not compress an explanation into symbols that the reader must decode.

Avoid decorative emphasis, title-case headings, inline labels that repeat the next words, and punctuation used in place of a sentence. Use sentence case, periods, and commas. A colon before a real list or example is useful. Use straight quotation marks in newly authored prose when format permits. Follow an established document's technical formatting when it carries meaning.

Keep technical terms when they are accurate and useful. Do not rewrite code, commands, CLI option spelling, machine JSON, schemas, citations, source quotations, filenames, fixture hashes, copyright notices, or license text for style. Keep precise native/media terms when a plainer substitute changes the claim. Avoid a broad search-and-replace pass over product outputs.

After editing, check that the text still tells the reader what changed, why it matters, what was tested, and what remains unverified. Delete a sentence that adds no project-specific fact, useful action, or necessary context. Retain a material caveat even when the shorter paragraph looks cleaner without it.

Adapted from pstack at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`, `pstack/skills/unslop/SKILL.md`. See [source hashes](sources.json) and [LICENSE](LICENSE).
