"""Freeze reviewed scene groups before fitting; keep oracle labels outside producer inputs."""
from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path

from . import isolate


def freeze(corpus_path: Path, groups_path: Path, output: Path) -> dict:
    catalog_bytes, grouping_bytes = corpus_path.read_bytes(), groups_path.read_bytes()
    catalog, grouping = json.loads(catalog_bytes), json.loads(grouping_bytes)
    candidates = {row['source_sha256']: row for row in catalog['samples'] if row['standard_calibration_candidate']}
    assignments = grouping.get('samples', {})
    if grouping.get('scene_groups_reviewed') is not True or set(assignments) != set(candidates):
        raise ValueError('reviewed grouping must assign every eligible sample exactly once')
    groups, image_groups = {}, {}
    for identity, assignment in assignments.items():
        group, split = assignment.get('scene_group'), assignment.get('split')
        if not isinstance(group, str) or not group.strip() or split not in {'calibration', 'holdout'}:
            raise ValueError('every sample requires a scene group and a valid split')
        if group in groups and groups[group] != split:
            raise ValueError('one scene group crosses calibration and holdout')
        groups[group] = split
        fingerprint = candidates[identity]['coded_primary_fingerprint']
        if fingerprint in image_groups and image_groups[fingerprint] != group:
            raise ValueError('the same coded primary belongs to different scene groups')
        image_groups[fingerprint] = group
    if set(groups.values()) != {'calibration', 'holdout'}:
        raise ValueError('both calibration and holdout scene groups are required')
    # Validate all source bytes before creating the output.
    for identity, row in candidates.items():
        if isolate.sha(Path(row['path']).read_bytes()) != identity:
            raise ValueError('catalog source identity changed')
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    producer, oracle = [], []
    for identity, row in sorted(candidates.items()):
        directory = output / 'producer' / identity
        projection = isolate.write_case(Path(row['path']), directory)
        assignment = assignments[identity]
        producer.append({'sample_id': identity, **assignment,
                         'input': str(directory / 'input.heic'),
                         'input_sha256': projection['producer_input_sha256'],
                         'native_equivalence_verified': False})
        oracle.append({'sample_id': identity, 'reference': row['path'], **assignment,
                       'reference_labels': {'style': row['style'], 'texture': row['texture'],
                                            'smartstyle_state': row['smartstyle_state']}})
    common = {'schema': 'styles-fidelity-split-v1', 'corpus_sha256': isolate.sha(catalog_bytes),
              'grouping_sha256': isolate.sha(grouping_bytes), 'scene_split_frozen': True,
              'prior_research_exposure': 'not established; this is a split for this experiment',
              'native_equivalence_verified': False, 'fitting_performed': False}
    (output / 'producer.json').write_text(json.dumps({**common, 'samples': producer}, indent=2) + '\n')
    (output / 'oracle.json').write_text(json.dumps({**common, 'samples': oracle}, indent=2) + '\n')
    summary = {**common, 'samples_per_split': dict(Counter(row['split'] for row in producer)),
               'scene_groups_per_split': dict(Counter(groups.values())),
               'producer_manifest_sha256': isolate.sha((output / 'producer.json').read_bytes()),
               'oracle_manifest_sha256': isolate.sha((output / 'oracle.json').read_bytes())}
    (output / 'split.json').write_text(json.dumps(summary, indent=2) + '\n')
    return summary


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus', type=Path, required=True)
    parser.add_argument('--groups', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    print(json.dumps(freeze(args.corpus, args.groups, args.output), indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
