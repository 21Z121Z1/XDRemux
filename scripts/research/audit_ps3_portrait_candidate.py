#!/usr/bin/env python3
import argparse
import json
import plistlib
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import patch_texture_style_heif_v3 as p


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument('candidate')
    ap.add_argument('manifest')
    ap.add_argument('output')
    args = ap.parse_args()

    path = Path(args.candidate)
    data = path.read_bytes()
    manifest = json.loads(Path(args.manifest).read_text())
    assert len(manifest['candidates']) == 1
    candidate = manifest['candidates'][0]
    assert candidate['payloadInvariant'] is True
    assert candidate['exifChanged'] is True
    assert candidate['newTextureItemID'] is not None

    top = list(p.boxes(data, 0, len(data)))
    meta = next(b for b in top if b['type'] == 'meta')
    children = list(p.boxes(data, meta['data_start'] + 4, meta['data_end']))
    by_type = {b['type']: b for b in children}
    _, infos = p.parse_iinf(data, by_type['iinf'])
    _, refs = p.parse_iref(data, by_type['iref'])
    iloc = p.parse_iloc(data, by_type['iloc'])
    idat = by_type.get('idat')
    entries = {e['id']: e for e in iloc['entries']}

    primary_id = p.parse_pitm(data, by_type['pitm']) if hasattr(p, 'parse_pitm') else None
    tmap_id = next(i['id'] for i in infos if i['type'] == 'tmap')
    semantic = next(i for i in infos if i['type'] == 'uri ' and b'tag:apple.com,2023:photo:metadata:styles\0' in i['raw'])
    texture = next(i for i in infos if i['type'] == 'uri ' and b'tag:apple.com,2026:photo:metadata:texture_styles\0' in i['raw'])

    semantic_targets = next(r['to'] for r in refs if r['type'] == 'cdsc' and r['from'] == semantic['id'])
    texture_targets = next(r['to'] for r in refs if r['type'] == 'cdsc' and r['from'] == texture['id'])
    assert texture_targets == semantic_targets and texture_targets
    assert tmap_id in semantic_targets
    if primary_id is not None:
        assert primary_id in semantic_targets

    texture_plist = plistlib.loads(p.item_payload(data, entries[texture['id']], idat))
    assert texture_plist['Version'] == 1
    assert texture_plist['HardwareModel'] == 'V63AP'
    for key in ('PortType', 'CaptureMode', 'CaptureType', 'FilmGrainSeed', 'TextureStylePeopleDataVersion', 'TextureStylePostProcessedPeopleData'):
        assert key in texture_plist

    exif_id = next(i['id'] for i in infos if i['type'] == 'Exif')
    exif_payload = p.item_payload(data, entries[exif_id], idat)
    note, _ = p.extract_makernote(exif_payload)
    _, note_entries = p.parse_apple_makernote(note)
    tag84 = next(raw for tag, _, raw in note_entries if tag == 84)
    maker = plistlib.loads(tag84)
    for key in ('TextureStylePreset', 'TextureStyleIntensity', 'TextureStyleGrain', 'TextureStyleRenderingVersion', 'TextureStyleOriginalInsteadOfReversibility'):
        assert key in maker
    for key in ('0', '1', '2', '3', '4', '5', '6', '7'):
        assert key in maker

    aux_to_tmap = [r for r in refs if r['type'] == 'auxl' and tmap_id in r['to']]
    semantic_aux = [r for r in aux_to_tmap if primary_id is None or primary_id in r['to']]
    assert len(semantic_aux) >= 3, 'expected at least Person/Skin/Sky semantic auxiliary links to primary+tmap'

    report = {
        'schema': 'xdremux-ps3-portrait-offline-audit-v1',
        'status': 'offline-structurally-complete',
        'candidate': str(path),
        'primaryItemID': primary_id,
        'toneMapItemID': tmap_id,
        'semanticStyleItemID': semantic['id'],
        'textureStyleItemID': texture['id'],
        'semanticAuxToPrimaryAndToneMapCount': len(semantic_aux),
        'sharedCdscTargets': semantic_targets,
        'payloadInvariant': True,
        'exifChanged': True,
        'makerNoteTextureKeys': {k: maker[k] for k in maker if str(k).startswith('TextureStyle')},
        'textureStyleInfo': texture_plist,
        'remainingGate': 'iOS 27 private parser acceptance and physical Photos edit/save/reopen/revert acceptance'
    }
    Path(args.output).write_text(json.dumps(report, indent=2, sort_keys=True))
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
