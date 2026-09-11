#!/usr/bin/env python3
from pathlib import Path
import re

path = Path('Sources/XDRemuxAppleFeatures/PhotographicStyles/ApplePhotographicStylesPipeline.swift')
source = path.read_text()

pattern = r'''(?s)(\s*let itemInfo = parseISOBMFFItemInfos\(data, iinf\)\n)\s*guard let toneMapID = itemInfo\.items\.first\(where: \{ \$0\.type == "tmap" \}\)\?\.itemID,\n\s*let ipmaBox = isobmffBoxes\(\n\s*in: data, start: iprp\.dataStart, end: iprp\.dataEnd\n\s*\)\.first\(where: \{ \$0\.type == "ipma" \}\) else \{\n\s*throw CLIError\.invalidContainer\("\\\(owner\) semantic merge item graph is incomplete"\)\n\s*\}'''
replacement = r'''\1            let primaryID = try parseISOBMFFPITM(data, pitm)
            let toneMapID: Int
            if let existingToneMapID = itemInfo.items.first(where: { $0.type == "tmap" })?.itemID {
                toneMapID = existingToneMapID
            } else if owner == "semantic scaffold" {
                // Hosted ImageIO may omit the temporary tmap. The real source HDR
                // remains authoritative and is still required to contain its tmap.
                toneMapID = primaryID
            } else {
                throw CLIError.invalidContainer("\(owner) semantic merge source has no tmap")
            }
            guard let ipmaBox = isobmffBoxes(
                in: data, start: iprp.dataStart, end: iprp.dataEnd
            ).first(where: { $0.type == "ipma" }) else {
                throw CLIError.invalidContainer("\(owner) semantic merge item graph is incomplete")
            }'''
source, count = re.subn(pattern, replacement, source, count=1)
if count != 1:
    raise SystemExit(f'tmap scaffold patch count={count}')

old_primary = '                primaryID: try parseISOBMFFPITM(data, pitm),'
if old_primary not in source:
    raise SystemExit('semantic scaffold primary return was not found')
source = source.replace(old_primary, '                primaryID: primaryID,', 1)

pattern_refs = r'''(?s)        for ref in scaffold\.refs where itemIDMap\[ref\.from\] != nil \{\n\s*outputRefs\.append\(ISOBMFFIRefEntry\(\n\s*type: ref\.type,\n\s*from: itemIDMap\[ref\.from\]!,\n\s*to: try ref\.to\.map\(mappedTarget\)\n\s*\)\)\n\s*\}'''
replacement_refs = '''        for ref in scaffold.refs where itemIDMap[ref.from] != nil {
            var targets = try ref.to.map(mappedTarget)
            if ref.type == "auxl", semanticImageIDs.contains(ref.from) {
                if !targets.contains(source.primaryID) { targets.append(source.primaryID) }
                if !targets.contains(source.toneMapID) { targets.append(source.toneMapID) }
            }
            outputRefs.append(ISOBMFFIRefEntry(
                type: ref.type,
                from: itemIDMap[ref.from]!,
                to: targets
            ))
        }'''
source, count = re.subn(pattern_refs, replacement_refs, source, count=1)
if count != 1:
    raise SystemExit(f'auxl restore patch count={count}')

path.write_text(source)
print('hosted semantic scaffold adapter applied; final source-HDR tmap requirement remains strict')
