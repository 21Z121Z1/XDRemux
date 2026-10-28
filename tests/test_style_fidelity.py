from pathlib import Path
import hashlib
import json
import plistlib
import struct
import tempfile
import unittest
import zipfile

import numpy as np

from research.styles_fidelity import corpus, hdr_exif, isolate, native, split


def hdr_note():
    # Independent wire fixture: HDR rationals, a forbidden tag 84, and trailing bytes.
    header = b'Apple iOS\0\0\x01MM\0\x03'
    entries = struct.pack('>HHII', 33, 10, 1, 56)
    entries += struct.pack('>HHII', 48, 10, 1, 64)
    entries += struct.pack('>HHII', 84, 7, 8, 72)
    return header + entries + b'\0' * 4 + struct.pack('>iiii', 7, 4, 1, 2) + b'WITHHELD-EXIF-AND-MAKERNOTE'


def box(kind, payload):
    return struct.pack('>I4s', len(payload) + 8, kind.encode()) + payload


def full(kind, version, flags, payload):
    return box(kind, bytes([version]) + flags.to_bytes(3, 'big') + payload)


def sample(style_version=16, *, baseline=8.0, texture='Standard', broken_reference=False, shared_exif=False, tmap=False):
    """Synthetic container, with recognizable withheld bytes, never a photo."""
    payloads = {
        1: b'compressed-primary', 2: b'compressed-gain',
        3: plistlib.dumps({'0': style_version, '1': b'WITHHELD-KEY1', '4': baseline}),
        4: b'WITHHELD-LINEAR-THUMBNAIL',
        5: plistlib.dumps({'Preset': texture, 'HardwareModel': 'test-only',
                           'TextureStylePostProcessedPeopleData': []}),
        6: hdr_exif.wrap_note(hdr_note()),
        7: b'WITHHELD-PERSON-MATTE',
    }
    kinds = {1: 'hvc1', 2: 'hvc1', 3: 'uri ', 4: 'hvc1', 5: 'uri ', 6: 'Exif', 7: 'hvc1'}
    if tmap:
        kinds[8] = 'tmap'
        payloads[8] = b'tone-map-transfer-parameters'
    uris = {3: isolate.STYLES_URI, 5: isolate.TEXTURE_URI}
    props = [full('ispe', 0, 0, struct.pack('>II', 64, 48)),
             box('hvcC', b'\x01' * 23),
             full('auxC', 0, 0, isolate.HDR_AUX.encode() + b'\0'),
             full('auxC', 0, 0, isolate.LINEAR_AUX.encode() + b'\0'),
             full('auxC', 0, 0, b'tag:apple.com,2026:photo:aux:semanticpersonmatte\0')]
    associations = {1: [(1, True), (2, True)], 2: [(1, True), (2, True), (3, False)],
                    4: [(1, True), (2, True), (4, False)], 7: [(1, True), (2, True), (5, False)]}
    infos = []
    for item, kind in kinds.items():
        tail = uris.get(item, '').encode() + b'\0' if item in uris else b''
        infos.append(full('infe', 3, 0 if item in (1, 8) else 1,
                          struct.pack('>IH4s', item, 0, kind.encode()) + b'name\0' + tail))
    refs = [('auxl', 2, [1]), ('cdsc', 3, [1]), ('auxl', 4, [1]),
            ('cdsc', 5, [1]), ('cdsc', 6, [1, 2] if shared_exif else [1]), ('auxl', 7, [1])]
    if broken_reference:
        refs.append(('dimg', 1, [999]))
    if tmap:
        refs.append(('dimg', 8, [1, 2]))
    idat = b''.join(payloads.values())
    offset = 0
    entries = []
    for item, payload in payloads.items():
        entries.append(struct.pack('>IHHHQQ', item, 1, 0, 1, offset, len(payload)))
        offset += len(payload)
    locations = full('iloc', 2, 0, b'\x88\x00' + struct.pack('>I', len(entries)) + b''.join(entries))
    ipma = struct.pack('>I', len(associations))
    for item, values in associations.items():
        ipma += struct.pack('>IB', item, len(values))
        ipma += b''.join(struct.pack('>H', index | (0x8000 if essential else 0)) for index, essential in values)
    children = full('pitm', 1, 0, struct.pack('>I', 1))
    children += full('iinf', 1, 0, struct.pack('>I', len(infos)) + b''.join(infos))
    children += locations
    children += full('iref', 1, 0, b''.join(box(kind, struct.pack('>IH', source, len(targets)) + b''.join(struct.pack('>I', t) for t in targets)) for kind, source, targets in refs))
    children += box('iprp', box('ipco', b''.join(props)) + full('ipma', 1, 1, ipma))
    children += box('idat', idat)
    return box('ftyp', b'heic\0\0\0\0mif1heic' + (b'MiHBtmap' if tmap else b'')) + full('meta', 0, 0, children)


class StyleFidelityTests(unittest.TestCase):
    def prepare_split(self, root, *, same_primary=False):
        rows, assignments = [], {}
        for index in range(2):
            payload = sample(baseline=8 + index)
            if index and not same_primary:
                payload = payload.replace(b'compressed-primary', b'different--primary')
            source = root / (str(index) + '.heic')
            source.write_bytes(payload)
            identity = isolate.sha(payload)
            rows.append({'source_sha256': identity, 'path': str(source),
                         'coded_primary_fingerprint': isolate.Graph(payload).image_fingerprint(1),
                         'standard_calibration_candidate': True, 'style': {'oracle': 'WITHHELD'},
                         'texture': {}, 'smartstyle_state': {}})
            assignments[identity] = {'scene_group': 'scene-' + str(index),
                                     'split': 'holdout' if index else 'calibration'}
        catalog = root / 'corpus.json'
        groups = root / 'groups.json'
        catalog.write_text(json.dumps({'samples': rows}))
        groups.write_text(json.dumps({'scene_groups_reviewed': True, 'samples': assignments}))
        return catalog, groups

    def test_frozen_split_separates_oracle_labels_and_pins_actual_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            catalog, groups = self.prepare_split(root)
            result = split.freeze(catalog, groups, root / 'frozen')
            producer = (root / 'frozen/producer.json').read_text()
            self.assertNotIn('WITHHELD', producer)
            self.assertNotIn('reference_labels', producer)
            for row in json.loads(producer)['samples']:
                self.assertEqual(isolate.sha(Path(row['input']).read_bytes()), row['input_sha256'])
                self.assertNotIn(b'WITHHELD', Path(row['input']).read_bytes())
            self.assertEqual(result['samples_per_split'], {'calibration': 1, 'holdout': 1})
            self.assertFalse(result['fitting_performed'])

    def test_same_coded_primary_cannot_leak_across_scene_groups(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            catalog, groups = self.prepare_split(root, same_primary=True)
            with self.assertRaisesRegex(ValueError, 'same coded primary'):
                split.freeze(catalog, groups, root / 'frozen')
            self.assertFalse((root / 'frozen').exists())

    def test_scene_group_cannot_cross_calibration_and_holdout(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            catalog, groups = self.prepare_split(root)
            value = json.loads(groups.read_text())
            for row in value['samples'].values():
                row['scene_group'] = 'one-scene'
            groups.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, 'scene group crosses'):
                split.freeze(catalog, groups, root / 'frozen')

    def test_changed_source_is_rejected_before_any_split_is_published(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            catalog, groups = self.prepare_split(root)
            (root / '0.heic').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'identity changed'):
                split.freeze(catalog, groups, root / 'frozen')
            self.assertFalse((root / 'frozen').exists())

    def test_isolation_keeps_coded_image_and_gain_but_removes_every_withheld_resource(self):
        original = sample()
        output, receipt = isolate.isolate_bytes(original)
        self.assertNotIn(b'WITHHELD', output)
        self.assertNotIn(isolate.STYLES_URI.encode(), output)
        self.assertNotIn(isolate.TEXTURE_URI.encode(), output)
        before, after = isolate.Graph(original), isolate.Graph(output)
        self.assertEqual(before.image_fingerprint(before.primary), after.image_fingerprint(after.primary))
        self.assertEqual(sorted(before.payloads[i] for i in (1, 2)),
                         sorted(after.payloads[i] for i in after.items if after.identities[i]['kind'] == 'hvc1'))
        self.assertEqual(receipt['retained_image_payloads'], 2)
        self.assertFalse(receipt['native_hdr_equivalence_verified'])

    def test_minimal_hdr_exif_keeps_exact_rationals_without_smartstyle_or_trailing_bytes(self):
        payload = hdr_exif.wrap_note(hdr_note())
        clean = hdr_exif.redact(payload)
        self.assertNotIn(b'WITHHELD', clean)
        self.assertIn(struct.pack('>iiii', 7, 4, 1, 2), clean)
        self.assertEqual(hdr_exif.redact(clean), clean)
        from PIL import Image
        exif = Image.Exif()
        exif.load(b'Exif\0\0' + clean[4:])
        self.assertEqual(set(exif.keys()), {271, 34665})
        self.assertEqual(set(exif.get_ifd(34665)), {37500})
        note = exif.get_ifd(34665)[37500]
        self.assertEqual(struct.unpack_from('>H', note, 14)[0], 2)
        self.assertEqual([struct.unpack_from('>H', note, 16 + i * 12)[0] for i in range(2)], [33, 48])

    def test_invalid_hdr_rational_is_rejected(self):
        note = hdr_note().replace(struct.pack('>ii', 7, 4), struct.pack('>ii', 7, 0))
        with self.assertRaisesRegex(ValueError, 'rational'):
            hdr_exif.redact(hdr_exif.wrap_note(note))

    def test_hdr_redaction_cannot_invent_an_apple_make(self):
        payload = hdr_exif.wrap_note(hdr_note()).replace(b'Apple\0', b'Other\0')
        with self.assertRaisesRegex(ValueError, 'source Apple make'):
            hdr_exif.redact(payload)

    def test_hdr_exif_associated_with_multiple_admitted_images_is_retained(self):
        output, receipt = isolate.isolate_bytes(sample(shared_exif=True))
        graph = isolate.Graph(output)
        exif_items = [i for i, v in graph.identities.items() if v['kind'] == 'Exif']
        self.assertEqual(len(exif_items), 1)
        self.assertEqual(set(graph.targets(exif_items[0], 'cdsc')), {1, 2})
        self.assertTrue(receipt['rebuilt_numeric_hdr_exif'])

    def test_projection_preserves_hdr_brands_and_visible_tone_map(self):
        original = sample(tmap=True)
        output, _ = isolate.isolate_bytes(original)
        before, after = isolate.Graph(original), isolate.Graph(output)
        self.assertEqual(before.file_type, after.file_type)
        tone_maps = [i for i, v in after.identities.items() if v['kind'] == 'tmap']
        self.assertEqual(len(tone_maps), 1)
        self.assertEqual(after.items[tone_maps[0]]['flags'], 0)
        self.assertEqual(after.targets(tone_maps[0], 'dimg'), [1, 2])

    def test_hdr_exif_ablation_is_stricter_and_cannot_copy_reference_state(self):
        output, receipt = isolate.isolate_bytes(sample(), keep_numeric_hdr_exif=False)
        self.assertFalse(receipt['rebuilt_numeric_hdr_exif'])
        self.assertFalse(any(v['kind'] == 'Exif' for v in isolate.Graph(output).identities.values()))
        self.assertNotIn(b'WITHHELD', output)

    def test_broken_dependency_cannot_be_silently_removed(self):
        with self.assertRaisesRegex(ValueError, 'missing|dependency'):
            isolate.isolate_bytes(sample(broken_reference=True))

    def test_same_primary_with_different_metadata_has_one_source_image_fingerprint(self):
        a, b = isolate.Graph(sample(baseline=8)), isolate.Graph(sample(baseline=30))
        self.assertEqual(a.image_fingerprint(a.primary), b.image_fingerprint(b.primary))
        self.assertNotEqual(hashlib.sha256(a.data).hexdigest(), hashlib.sha256(b.data).hexdigest())

    def test_plist_labels_do_not_enter_producer_input(self):
        output, _ = isolate.isolate_bytes(sample(baseline=39.923))
        self.assertNotIn(plistlib.dumps({'0': 16, '1': b'WITHHELD-KEY1', '4': 39.923}), output)

    def test_catalog_deduplicates_bytes_and_retains_both_archive_sources(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths = []
            for name in ('first.zip', 'second.zip'):
                path = root / name
                with zipfile.ZipFile(path, 'w') as archive:
                    archive.writestr('nested/sample.heic', sample())
                paths.append(path)
            report = corpus.catalog(paths, root / 'catalog')
            self.assertEqual(len(report['samples']), 1)
            self.assertEqual(len(report['samples'][0]['sources']), 2)
            self.assertEqual(report['summary']['archive_heif_entries'], 2)
            self.assertEqual(report['summary']['unique_heif_files'], 1)

    def test_catalog_rejects_existing_output_without_touching_it(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'retain').write_bytes(b'previous')
            with self.assertRaises(FileExistsError):
                corpus.catalog([], root)
            self.assertEqual((root / 'retain').read_bytes(), b'previous')

    def test_zip_traversal_names_are_recorded_but_never_extracted_as_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / 'input.zip'
            with zipfile.ZipFile(archive, 'w') as stream:
                stream.writestr('../../escape.heic', sample())
            report = corpus.catalog([archive], root / 'catalog')
            self.assertFalse((root / 'escape.heic').exists())
            self.assertEqual(len(report['samples']), 1)
            self.assertEqual(len(list((root / 'catalog' / 'originals').glob('*.heic'))), 1)

    def test_incomplete_style_metadata_cannot_be_admitted_as_standard(self):
        facts = corpus.inspect_bytes(sample())
        self.assertEqual(facts['style']['version'], 16)
        self.assertIsNone(facts['standard_semantic_state'])
        self.assertFalse(facts['standard_calibration_candidate'])

    def test_metadata_diagnostic_reports_native_values_outside_product_bounds(self):
        facts = corpus.inspect_bytes(sample(baseline=39.923))
        self.assertTrue(facts['style']['baseline_outside_current_product_range'])
        self.assertEqual(facts['style']['baseline_exposure'], 39.923)

    def test_hdr_descriptor_redaction_does_not_copy_other_xmp_fields(self):
        raw = ('<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" '
               'xmlns:hdr="' + isolate.HDR_NAMESPACE + '" xmlns:secret="https://test.invalid/private/">'
               '<rdf:Description hdr:HDRGainMapVersion="65536" hdr:HDRGainMapHeadroom="4" '
               'secret:Styles="WITHHELD" /></rdf:RDF>').encode()
        redacted = isolate._hdr_xmp(raw)
        self.assertIn(b'HDRGainMapHeadroom', redacted)
        self.assertNotIn(b'WITHHELD', redacted)
        self.assertNotIn(b'private', redacted)
        for value in ('WITHHELD', 'nan', 'inf'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                isolate._hdr_xmp(raw.replace(b'Headroom="4"', ('Headroom="' + value + '"').encode()))

    def test_native_buffer_comparison_counts_real_errors_and_rejects_nonfinite(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            reference, candidate = root / 'a.f32', root / 'b.f32'
            data = np.array([[[.25, .5, 1, 1]]], dtype='<f4')
            reference.write_bytes(data.tobytes())
            candidate.write_bytes(data.tobytes())
            self.assertTrue(native.compare_buffers(reference, candidate, 1, 1)['passed'])
            altered = data.copy()
            altered[0, 0, 0] = .3
            candidate.write_bytes(altered.tobytes())
            facts = native.compare_buffers(reference, candidate, 1, 1)
            self.assertFalse(facts['passed'])
            self.assertEqual(facts['different_float32_components'], 1)
            altered[0, 0, 0] = np.nan
            candidate.write_bytes(altered.tobytes())
            self.assertFalse(native.compare_buffers(reference, candidate, 1, 1)['passed'])
            candidate.write_bytes(b'\0')
            with self.assertRaisesRegex(ValueError, 'buffer length'):
                native.compare_buffers(reference, candidate, 1, 1)


if __name__ == '__main__':
    unittest.main()
