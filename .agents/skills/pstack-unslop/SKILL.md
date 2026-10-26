---
name: pstack-unslop
description: Edit authored XDRemux prose to remove vague claims, AI phrasing, unnecessary jargon, and repetitive framing while preserving meaning, evidence, identifiers, and intended tone. Use for explanations, documentation, review reports, and requested writing cleanup.
---

# Plain, evidence-backed prose

State the outcome early. Use familiar words, active verbs, and connected sentences. Preserve the intended meaning, source attribution, evidence limits, and the reader's required technical detail. Follow the repository policy for canonical English documentation and matching Chinese documentation.

## Replace vague claims

Remove filler, generic praise, chatbot closers, decorative emoji, vague attribution, and inflated claims. For an attribution such as "experts believe", name the source or remove the attribution. Support abstract claims with a mechanism, observed behavior, or measured value. When shortening text, keep an inference distinct from an established fact.

## Use direct sentences

Prefer "use", "help", "is", and "has" to elaborate synonyms. Avoid forced groups of three, changing synonyms for the same concept, false ranges, and "not just X but Y" framing. Remove elaborate metaphors and stock conclusions.

Keep one main idea in each sentence. Retain the articles and verbs needed for natural grammar. Do not compress an explanation into symbols that the reader must decode.

## Keep formatting functional

Use sentence case for headings. Remove decorative emphasis and inline labels that repeat the next words. Use periods and commas to separate ideas. Do not use punctuation as a substitute for a sentence. Use a colon before a list or example when it clarifies the structure.

Use straight quotation marks in new prose when the format permits. Preserve established technical formatting when it carries meaning.

## Preserve technical and source content

Keep accurate technical terms that help the reader. Preserve precise native and media terms when a plainer substitute would change the claim.

Do not change code, commands, CLI option spelling, machine JSON, or schemas for style. Preserve citations, source quotations, filenames, fixture hashes, copyright notices, and license text. Do not apply broad search-and-replace changes to product outputs.

## Check the edited text

Confirm that the text states what changed, why it matters, what was tested, and what remains unverified. Remove sentences that add no project-specific fact, useful action, or necessary context. Retain every material caveat, even if removing it would shorten the paragraph.

Adapted from pstack at `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`, `pstack/skills/unslop/SKILL.md`. See [source hashes](sources.json) and [LICENSE](LICENSE).
