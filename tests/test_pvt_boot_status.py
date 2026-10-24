"""Replay monitor output; these tests do not establish native readiness."""
import ast
import copy
from pathlib import Path
import subprocess
import unittest
from unittest.mock import Mock

from scripts.apple.pvt_native_import import check_native_import as product
from scripts.apple.pvt_native_import.boot_status import observe_boot_status


OWNED = '481E73B6-B8CB-49EC-9C8F-42C86B91A2E2'
FAILED = (Path(__file__).parent / 'fixtures/pvt_boot_status/data-migration-failed.stdout').read_bytes()
# Synthetic success in the observed simctl format, not a native receipt.
SUCCESS = (f'Monitoring boot status for owned ({OWNED}).\n'
           '[2026-10-23 08:00:00 +0000] Status=4294967295, isTerminal=YES, Elapsed=00:54.\n'
           '\tFinished\n').encode()


class BootStatusTests(unittest.TestCase):
    def test_actual_zero_exit_failure_is_not_a_successful_boot(self):
        observation = observe_boot_status(FAILED, OWNED)
        self.assertEqual(observation['state'], 'blocked')
        self.assertEqual(observation['status'], 3)
        self.assertTrue(observation['terminal'])
        self.assertEqual(observation['message'], 'Data Migration Failed')
        self.assertEqual(observation['elapsed'], '00:54')

    def test_recognized_success_terminal_can_proceed_to_inventory(self):
        self.assertEqual(observe_boot_status(SUCCESS, OWNED)['state'], 'completed')

    def test_unknown_incomplete_contradictory_or_foreign_output_fails_closed(self):
        cases = [b'', b'\xff', b'Device already booted, nothing to do.\n',
                 SUCCESS.replace(b'4294967295', b'99'), SUCCESS.replace(b'YES', b'NO'),
                 SUCCESS.replace(b'Finished', b'Unknown'), SUCCESS.replace(OWNED.encode(), b'0' * 36),
                 SUCCESS + b'Unexpected later output\n', FAILED + SUCCESS,
                 SUCCESS.splitlines(keepends=True)[0] + SUCCESS,
                 SUCCESS.splitlines(keepends=True)[0] + SUCCESS.replace(OWNED.encode(), b'0' * 36),
                 SUCCESS.replace(b'Status=', b'NewStatus='), SUCCESS.rsplit(b'\n', 2)[0]]
        for stdout in cases:
            with self.subTest(stdout=stdout[-100:]):
                self.assertEqual(observe_boot_status(stdout, OWNED)['state'], 'blocked')

    def test_product_refuses_failed_boot_before_install_and_retains_observation(self):
        # Execute the product's real boot-to-install block, avoiding unrelated
        # compilation and media setup on this non-Apple host.
        tree = ast.parse(Path(product.__file__).read_text())
        main = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == 'main')
        body = next(node for node in main.body if isinstance(node, ast.Try)).body
        start = next(i for i, node in enumerate(body) if isinstance(node, ast.Assign)
                     and isinstance(node.value, ast.Call) and node.value.args
                     and isinstance(node.value.args[0], ast.Constant) and node.value.args[0].value == 'boot-owned-status')
        stop = next(i for i, node in enumerate(body) if isinstance(node, ast.Expr)
                    and isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Name)
                    and node.value.func.id == 'install_owned_reader')
        code = compile(ast.fix_missing_locations(ast.Module(body=copy.deepcopy(body[start:stop + 1]), type_ignores=[])),
                       product.__file__, 'exec')
        for stdout, accepted in [(FAILED, False), (b'', False), (SUCCESS, True)]:
            with self.subTest(accepted=accepted):
                report, install = {}, Mock()
                run = Mock(return_value=subprocess.CompletedProcess([], 0, stdout, b''))
                context = {**vars(product), 'report': report, 'owned_udid': OWNED, 'run': run,
                           'bundle': 'owned.reader', 'app': Path('/owned/app'), 'work': Path('/owned'),
                           'install_owned_reader': install}
                if accepted:
                    exec(code, context)
                    install.assert_called_once()
                else:
                    with self.assertRaises(product.Blocked): exec(code, context)
                    install.assert_not_called()
                self.assertEqual(report['boot_observation']['state'], 'completed' if accepted else 'blocked')
                run.assert_called_once_with('boot-owned-status', ['xcrun', 'simctl', 'bootstatus', OWNED, '-b'], timeout=180)


if __name__ == '__main__':
    unittest.main()
