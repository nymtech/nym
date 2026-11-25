<!-- Draft template for one property in the assurance view (add-codex-assurance A4). Placeholders in braces are record fields. The field names are provisional. A view is plain Markdown (add-codex-snippets D16), so a cited record appears as its text and a link, not as a component. -->

### {property.id}: {property.name}

{property.text}

Source: [{property.recordId}]({property.link})

**Realised by:** {requirements.length} requirements. {requirements.withCode} of them have code that realises them.

| Requirement | Level | Status | Warrant | Code | Canary |
|---|---|---|---|---|---|
{for requirement in requirements}| [{requirement.title}]({requirement.link}) | {requirement.level} | {requirement.status} | {requirement.warrant} | {requirement.code or "none"} | {requirement.canary or "none"} |
{end}

{if requirements.length == 0}No requirement realises this property. The open gaps section lists it.{end}
