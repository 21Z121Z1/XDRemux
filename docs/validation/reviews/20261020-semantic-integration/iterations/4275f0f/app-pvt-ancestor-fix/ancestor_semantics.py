"""Real POSIX filesystem model; neither executes nor predicts Darwin Foundation."""
import json
import os
from pathlib import Path
import tempfile


def contains_package(path):
    return any(Path(part).suffix.lower() == '.pvt' for part in path.parts)


def whole(path, mode):
    try:
        return Path(os.path.realpath(path, strict=mode == 'requires-existing-leaf'))
    except FileNotFoundError:
        return path


def old_guard(path, mode):
    for candidate in [path, whole(path, mode)]:
        if contains_package(candidate.parent):
            return True
    names_package = path.suffix.lower() == '.pvt' or whole(path, mode).suffix.lower() == '.pvt'
    return names_package and os.path.lexists(path)


def ancestor_guard(path):
    if contains_package(path.parent):
        return True
    candidate = path
    while candidate != candidate.parent:
        if os.path.lexists(candidate) and contains_package(Path(os.path.realpath(candidate, strict=True))):
            return True
        candidate = candidate.parent
    return False


results = {'host': 'POSIX Linux', 'darwin_foundation_execution': False,
           'limits': 'Both whole-path behaviors are hypotheses. Only the next native App run can identify actual Darwin behavior and the original failed subcase.',
           'scenarios': []}
with tempfile.TemporaryDirectory(prefix='pvt-ancestor-model-') as directory:
    root = Path(directory)
    package = root / 'existing.PVT'
    interior = package / 'interior'
    interior.mkdir(parents=True)
    (package / 'member.heic').write_bytes(b'unchanged member')
    (interior / 'inside.heic').write_bytes(b'unchanged interior member')
    alias, inner, chain = [root / name for name in ['package-alias', 'interior-alias', 'chain-alias']]
    alias.symlink_to(package, target_is_directory=True)
    inner.symlink_to(interior, target_is_directory=True)
    chain.symlink_to(inner, target_is_directory=True)
    scenarios = [('direct-package', package), ('direct-missing-child', package / 'nested'),
                 ('alias-package', alias), ('alias-missing-child', alias / 'missing'),
                 ('alias-missing-multilevel', alias / 'missing/deeper/still-missing'),
                 ('alias-interior', inner), ('alias-interior-missing-multilevel', inner / 'missing/deeper'),
                 ('alias-chain-missing-multilevel', chain / 'missing/deeper')]
    for name, parent in scenarios:
        for extension in ['heic', 'pvt']:
            path = parent / ('source.' + extension)
            record = {'case': name, 'extension': extension, 'new_guard': ancestor_guard(path),
                      'old_whole_path_requires_existing_leaf': old_guard(path, 'requires-existing-leaf'),
                      'old_whole_path_resolves_existing_prefix': old_guard(path, 'resolves-existing-prefix')}
            assert record['new_guard'], record
            results['scenarios'].append(record)
    ordinary = root / 'ordinary'
    ordinary.mkdir()
    ordinary_alias = root / 'ordinary-alias'
    ordinary_alias.symlink_to(ordinary, target_is_directory=True)
    for path in [ordinary / 'fresh.heic', ordinary / 'fresh.pvt', ordinary_alias / 'missing/deeper/fresh.heic', ordinary_alias / 'missing/deeper/fresh.pvt']:
        assert not ancestor_guard(path)
    # Current nodes must also be checked, including ordinary filenames pointing
    # into the middle of an opaque package rather than to its outer .pvt node.
    member_alias = root / 'plain.heic'
    member_alias.symlink_to(interior / 'inside.heic')
    assert ancestor_guard(member_alias)
    assert (package / 'member.heic').read_bytes() == b'unchanged member'
    assert (interior / 'inside.heic').read_bytes() == b'unchanged interior member'
    assert sorted(path.name for path in package.iterdir()) == ['interior', 'member.heic']
results['positive_controls'] = 4
results['existing_current_alias_to_interior_member_blocked'] = True
results['package_bytes_and_member_sets_unchanged'] = True
Path(__file__).with_name('ancestor-semantics-result.json').write_text(json.dumps(results, indent=2) + '\n')
print(json.dumps({'modeled_guard_cases': len(results['scenarios']), 'positive_controls': 4,
                  'native_execution': False, 'conditional_old_guard_failures': sum(not r['old_whole_path_requires_existing_leaf'] for r in results['scenarios'])}))
