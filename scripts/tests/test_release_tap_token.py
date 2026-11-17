"""Run the permission probe offline; no token or network access is required."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).parents[1] / 'check-tap-token.sh'


class TapTokenTests(unittest.TestCase):
    def scenario(self, failure=''):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'bin').mkdir()
            (root / 'state').write_text(json.dumps({'branch': False, 'pr': False}))
            fake = f'''#!{sys.executable}
import json,os,sys
from pathlib import Path
args=sys.argv[1:]; name=Path(sys.argv[0]).name
root=Path(os.environ['PROBE_ROOT']); state=json.loads((root/'state').read_text())
with (root/'calls').open('a') as stream: stream.write(json.dumps([name,*args])+'\\n')
failure=os.environ['PROBE_FAILURE']
if name=='git':
    if args[:2]==['remote','get-url']: print('https://github.com/keroway/homebrew-tap')
    elif args[:1]==['push']:
        if '--delete' in args:
            if failure=='delete': sys.exit(1)
            state['branch']=False
        else:
            if failure=='push': sys.exit(1)
            state['branch']=True
    elif args[0] not in ['config','switch','add','commit','status','ls-remote']: sys.exit(99)
elif args[:2]==['pr','create']:
    if failure=='create': sys.exit(1)
    state['pr']=True
    (root/'state').write_text(json.dumps(state))
    if failure=='create-unknown': sys.exit(1)
    print('https://github.com/keroway/homebrew-tap/pull/123')
elif args[:2]==['pr','close']:
    if failure=='close': sys.exit(1)
    state['pr']=False
elif args[:2]==['repo','view']: print('main')
elif args[:1]==['api']:
    if failure=='lookup': sys.exit(1)
    if state['pr']: print('123')
else: sys.exit(99)
(root/'state').write_text(json.dumps(state))
'''
            for name in ['git', 'gh']:
                path = root / 'bin' / name
                path.write_text(fake)
                path.chmod(0o755)
            env = {**os.environ, 'PATH': str(root / 'bin') + os.pathsep + os.environ['PATH'],
                   'GH_TOKEN': 'offline-fixture', 'GITHUB_RUN_ID': '123', 'GITHUB_RUN_ATTEMPT': '1',
                   'PROBE_ROOT': str(root), 'PROBE_FAILURE': failure}
            if failure == 'invalid-run': env['GITHUB_RUN_ID'] = '../other'
            result = subprocess.run(['bash', str(SCRIPT)], cwd=root, env=env, capture_output=True, text=True)
            calls = [json.loads(line) for line in (root / 'calls').read_text().splitlines()] if (root / 'calls').exists() else []
            return result, json.loads((root / 'state').read_text()), calls

    def test_success_creates_and_removes_probe(self):
        result, state, calls = self.scenario()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(state, {'branch': False, 'pr': False})
        self.assertIn('verified', result.stdout)
        self.assertTrue(any(call[:3] == ['gh', 'pr', 'create'] and '--draft' in call for call in calls))
        self.assertTrue(any('[skip ci]' in ' '.join(call) for call in calls))
        self.assertFalse(any('--force' in call or 'merge' in call for call in calls))
        self.assertTrue(all(any('keroway/homebrew-tap' in arg for arg in call) for call in calls if call[0] == 'gh'))

    def test_pr_failure_deletes_pushed_branch(self):
        result, state, _ = self.scenario('create')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(state, {'branch': False, 'pr': False})

    def test_close_failure_preserves_branch_and_fails(self):
        result, state, _ = self.scenario('close')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(state, {'branch': True, 'pr': True})
        self.assertIn('cleanup failed', result.stderr)

    def test_delete_failure_is_not_success(self):
        result, state, _ = self.scenario('delete')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(state, {'branch': True, 'pr': False})

    def test_push_failure_never_creates_pr(self):
        result, state, calls = self.scenario('push')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(state, {'branch': False, 'pr': False})
        self.assertFalse(any(call[:3] == ['gh', 'pr', 'create'] for call in calls))

    def test_lost_create_response_still_closes_pr_before_deleting_branch(self):
        result, state, calls = self.scenario('create-unknown')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(state, {'branch': False, 'pr': False})
        close = next(i for i, call in enumerate(calls) if call[:3] == ['gh', 'pr', 'close'])
        delete = next(i for i, call in enumerate(calls) if '--delete' in call)
        self.assertLess(close, delete)

    def test_uncertain_lookup_keeps_branch_and_reports_failure(self):
        result, state, calls = self.scenario('lookup')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(state, {'branch': True, 'pr': True})
        self.assertFalse(any('--delete' in call for call in calls))
        self.assertIn('cleanup uncertain', result.stderr)

    def test_invalid_run_id_stops_before_git(self):
        result, _, calls = self.scenario('invalid-run')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, [])
