"""Run the actual bump script with isolated fake git/gh executables."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).parents[1] / 'release-homebrew.sh'


class HomebrewRetryTests(unittest.TestCase):
    def scenario(self, branch=False, pr=False, mismatch=False):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'bin').mkdir(); (root / 'Formula').mkdir()
            (root / 'Formula/tdsl.rb').write_text('verified formula\n')
            fake = f'''#!{sys.executable}
import json,os,sys
from pathlib import Path
args=sys.argv[1:]
name=Path(sys.argv[0]).name
with open(os.environ['CALL_LOG'],'a') as stream: stream.write(json.dumps([name,*args])+'\\n')
if name=='git' and args[:2]==['ls-remote','--heads'] and os.environ['HAS_BRANCH']=='1': print('abc refs/heads/bump-tdsl-2.1.0')
if name=='git' and args[:1]==['show']: print('different' if os.environ['MISMATCH']=='1' else 'verified formula')
if name=='git' and args[:1]==['diff']: sys.exit(1)
if name=='gh' and args[:2]==['pr','list'] and os.environ['HAS_PR']=='1': print('https://github.com/keroway/homebrew-tap/pull/123')
'''
            for name in ['git','gh']:
                path = root / 'bin' / name; path.write_text(fake); path.chmod(0o755)
            env = {**os.environ, 'PATH': str(root / 'bin') + os.pathsep + os.environ['PATH'], 'TARGET_TAG':'v2.1.0', 'CALL_LOG':str(root/'calls'), 'HAS_BRANCH':str(int(branch)), 'HAS_PR':str(int(pr)), 'MISMATCH':str(int(mismatch))}
            result = subprocess.run(['bash',str(SCRIPT)],cwd=root,env=env,capture_output=True,text=True)
            calls = [json.loads(line) for line in (root/'calls').read_text().splitlines()]
            return result.returncode, calls

    def test_new_branch_and_pr(self):
        code,calls = self.scenario()
        self.assertEqual(code,0)
        self.assertEqual(sum(call[:2]==['git','push'] for call in calls),1)
        self.assertEqual(sum(call[:3]==['gh','pr','create'] for call in calls),1)
        self.assertFalse(any('--force' in call for call in calls))

    def test_interrupted_branch_reuses_commit(self):
        code,calls = self.scenario(branch=True)
        self.assertEqual(code,0)
        self.assertFalse(any(call[:2]==['git','push'] for call in calls))
        self.assertTrue(any(call[:3]==['gh','pr','create'] for call in calls))

    def test_existing_pr_is_reused(self):
        code,calls = self.scenario(branch=True,pr=True)
        self.assertEqual(code,0)
        self.assertFalse(any(call[:2]==['git','push'] or call[:3]==['gh','pr','create'] for call in calls))

    def test_conflicting_branch_fails_without_push(self):
        code,calls = self.scenario(branch=True,mismatch=True)
        self.assertNotEqual(code,0)
        self.assertFalse(any(call[:2]==['git','push'] or call[:3]==['gh','pr','create'] for call in calls))


if __name__=='__main__': unittest.main()
