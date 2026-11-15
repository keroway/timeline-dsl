"""Offline fixtures only: no registry publication or external API calls."""
import base64
import hashlib
import importlib.util
import io
import json
import os
import sys
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location('release', Path(__file__).parents[1] / 'release.py')
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)
TAG = 'v2.1.0'
SHA = 'a' * 40


def tar(files):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w:gz') as archive:
        for name, value in files.items():
            raw = json.dumps(value).encode()
            entry = tarfile.TarInfo(name); entry.size = len(raw)
            archive.addfile(entry, io.BytesIO(raw))
    return buffer.getvalue()


def assets_fixture():
    assets = {}
    proofs = {}
    for name in (*r.CLI, f'tdsl-wasm-{TAG}.tar.gz', 'timeline-dsl-2.1.0.vsix'):
        digest = hashlib.sha256(name.encode()).hexdigest()
        assets[name] = {'digest': 'sha256:' + digest}
        assets[name + '.provenance.json'] = {'browser_download_url': name + '.json'}
        proofs[name + '.json'] = json.dumps({'version': '2.1.0', 'sha': SHA, 'name': name, 'sha256': digest}).encode()
    return assets, proofs


class ReleaseTests(unittest.TestCase):
    def test_tag_validation(self):
        for tag in ['main', 'v1.2', 'v01.2.3', 'v1.2.3;echo bad', '../tag', '1.2.3']:
            with self.assertRaises(ValueError): r.validate_tag(tag)
        self.assertEqual(r.validate_tag(TAG), '2.1.0')

    def test_verified_tag_versions_and_sha(self):
        def git(*args):
            if args[1] == 'rev-parse': return 'b'*40 if args[-1].startswith('b'*40) else SHA
            path = args[-1].split(':', 1)[1]
            if path == 'Cargo.toml': return '[workspace.package]\nversion="2.1.0"'
            if path.endswith('Cargo.toml'): return '[package]\nversion.workspace=true'
            if path.endswith('package-lock.json'): return json.dumps({'version':'2.1.0','packages':{'':{'version':'2.1.0'}}})
            return '{"version":"2.1.0"}'
        with patch.object(r, 'command', side_effect=git):
            self.assertEqual(r.verify(TAG, SHA), SHA)
            with self.assertRaises(ValueError): r.verify(TAG, 'b' * 40)
            with self.assertRaises(ValueError): r.verify('v2.2.0')

    def test_release_notes_preserved_and_update_idempotent(self):
        notes = 'Human release notes\n\nMigration example\n'
        first = r.merge_body(notes, 'old table')
        second = r.merge_body(first, 'new table')
        self.assertIn(notes.rstrip(), second)
        self.assertEqual(second.count(r.START), 1)
        self.assertNotIn('old table', second)
        self.assertEqual(r.merge_body(second, 'new table'), second)
        with self.assertRaises(ValueError): r.merge_body(r.START, 'table')

    def test_asset_existing_requires_matching_provenance(self):
        assets, proofs = assets_fixture()
        with patch.object(r, 'download', side_effect=proofs.get):
            self.assertTrue(r.asset_present(assets, r.CLI[0], TAG, SHA))
            with self.assertRaises(r.Unconfirmed): r.asset_present(assets, r.CLI[0], TAG, 'b'*40)
            assets[r.CLI[0]]['digest'] = 'sha256:bad'
            with self.assertRaises(r.Unconfirmed): r.asset_present(assets, r.CLI[0], TAG, SHA)
        self.assertFalse(r.asset_present({}, r.CLI[0], TAG, SHA))

    def test_npm_existing_and_missing(self):
        metadata = json.dumps({'name':'@keroway/tdsl-wasm','version':'2.1.0','dist':{'tarball':'https://registry.npmjs.org/a.tgz'}}).encode()
        content = tar({'package/release-source.json': {'version':'2.1.0','sha':SHA}, 'package/package.json': {'name':'@keroway/tdsl-wasm', 'version':'2.1.0'}})
        parsed = json.loads(metadata); parsed['dist']['integrity'] = 'sha512-' + base64.b64encode(hashlib.sha512(content).digest()).decode(); metadata = json.dumps(parsed).encode()
        with patch.object(r, 'download', side_effect=[metadata, content]):
            self.assertTrue(r.registry_present('npm', TAG, SHA))
        with patch.object(r, 'download', return_value=None):
            self.assertFalse(r.registry_present('npm', TAG, SHA))
        with patch.object(r, 'download', side_effect=r.Unconfirmed('API down')):
            with self.assertRaises(r.Unconfirmed): r.registry_present('npm', TAG, SHA)

    def test_crate_checks_source_and_checksum(self):
        data = tar({'tdsl-core-2.1.0/.cargo_vcs_info.json': {'git':{'sha1':SHA}}})
        metadata = json.dumps({'version':{'num':'2.1.0','crate':'tdsl-core','checksum':hashlib.sha256(data).hexdigest()}}).encode()
        with patch.object(r, 'download', side_effect=[metadata, data]):
            self.assertTrue(r.registry_present('tdsl-core', TAG, SHA))
        with patch.object(r, 'download', side_effect=[metadata, data]):
            with self.assertRaises(r.Unconfirmed): r.registry_present('tdsl-core', TAG, 'b'*40)

    def test_marketplace_provenance(self):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, 'w') as archive:
            archive.writestr('extension/package.json',json.dumps({'publisher':'keroway','name':'timeline-dsl','version':'2.1.0'}))
            archive.writestr('extension/release-source.json',json.dumps({'sha':SHA,'version':'2.1.0'}))
        with patch.object(r, 'download', return_value=buffer.getvalue()):
            self.assertTrue(r.registry_present('Marketplace', TAG, SHA))

    def test_all_success_partial_failure_running_and_unknown(self):
        assets, proofs = assets_fixture()
        for active in [False, True, None]:
            with patch.object(r,'release',return_value={'id':1}), patch.object(r,'assets_for',return_value=assets), patch.object(r,'download',side_effect=proofs.get), patch.object(r,'active_release',return_value=active), patch.object(r,'homebrew_state',return_value=(True,'merged')):
                with patch.object(r,'registry_present',return_value=True):
                    _, table = r.inventory(TAG, SHA, 1)
                    self.assertEqual(table.count('| 公開済み |'), 14)
                def probe(channel, *_):
                    if channel == 'npm': return False
                    if channel == 'tdsl-core': raise r.Unconfirmed('failure')
                    return True
                with patch.object(r,'registry_present',side_effect=probe):
                    _, table = r.inventory(TAG, SHA, 1)
                    self.assertIn('| tdsl-core | 確認不能 |', table)
                    self.assertIn(f'| npm | {r.status_label(False, active)} |', table)
                    self.assertIn('| Marketplace | 公開済み |', table)

    def test_other_workflow_running_current_run_excluded(self):
        runs = {'workflow_runs':[{'id':10,'display_title':f'Release {TAG}','status':'in_progress'},{'id':11,'display_title':f'VS Code {TAG}','status':'queued'}]}
        with patch.object(r, 'gh', side_effect=lambda path: {'jobs':[{'name':'Build VSIX','status':'queued'}]} if '/jobs?' in path else runs):
            self.assertTrue(r.active_release(TAG, 10))
        runs['workflow_runs'].pop()
        with patch.object(r,'gh',return_value=runs):
            self.assertFalse(r.active_release(TAG, 10))

    def test_homebrew_pr_is_not_published(self):
        assets, proofs = assets_fixture()
        text = '\n'.join(f'url "https://github.com/{r.REPO}/releases/download/{TAG}/{name}"\nsha256 "{assets[name]["digest"][7:]}"' for name in r.CLI[:4])
        def api(path, *args):
            if path == f'repos/{r.TAP}': return {'default_branch':'main'}
            return [{'head':{'sha':'b'*40}, 'html_url':'https://github.com/keroway/homebrew-tap/pull/123'}]
        with patch.object(r,'gh',side_effect=api), patch.object(r,'download',side_effect=proofs.get), patch.object(r,'formula_at',side_effect=['old formula',text]):
            published, detail = r.homebrew_state(TAG, SHA, assets)
            self.assertFalse(published)
            self.assertIn('PR created; merge pending', detail)
        with patch.object(r,'gh',side_effect=api), patch.object(r,'download',side_effect=proofs.get), patch.object(r,'formula_at',return_value=text):
            self.assertTrue(r.homebrew_state(TAG, SHA, assets)[0])

    def test_retry_assets_skips_existing_without_upload(self):
        assets, proofs = assets_fixture()
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / r.CLI[0]; path.write_bytes(b'rebuild')
            with patch.object(r,'release',return_value={'id':1}), patch.object(r,'assets_for',return_value=assets), patch.object(r,'download',side_effect=proofs.get), patch.object(r,'command') as cmd:
                r.publish_assets(TAG, SHA, [str(path)])
                cmd.assert_not_called()

    def test_stamp_preserves_version_and_rejects_mismatch(self):
        with tempfile.TemporaryDirectory() as folder:
            package = Path(folder)
            manifest = {'name':'@keroway/tdsl-wasm','version':'2.1.0','files':['module.wasm']}
            (package/'package.json').write_text(json.dumps(manifest))
            with patch.dict(os.environ, {'TARGET_TAG':TAG,'TARGET_SHA':SHA}), patch.object(sys,'argv',['release.py','stamp',folder]):
                r.main()
                self.assertEqual(json.loads((package/'release-source.json').read_text()), {'sha':SHA,'version':'2.1.0'})
                self.assertEqual(json.loads((package/'package.json').read_text())['version'], '2.1.0')
                manifest['version'] = '2.0.0'
                (package/'package.json').write_text(json.dumps(manifest))
                with self.assertRaises(ValueError): r.main()
                self.assertEqual(json.loads((package/'package.json').read_text())['version'], '2.0.0')

    def test_unknown_existing_registry_stops_pending(self):
        with patch.dict(os.environ,{'TARGET_TAG':TAG,'TARGET_SHA':SHA}), patch.object(sys,'argv',['release.py','pending','npm']), patch.object(r,'registry_present',side_effect=r.Unconfirmed('missing provenance')), patch.object(r,'output') as output:
            with self.assertRaises(r.Unconfirmed): r.main()
            output.assert_not_called()

    def test_release_api_failure_produces_unknown_assets(self):
        with patch.object(r,'release',side_effect=r.Unconfirmed('API down')), patch.object(r,'active_release',return_value=False), patch.object(r,'registry_present',return_value=False):
            info, table = r.inventory(TAG,SHA,1)
            self.assertIsNone(info)
            self.assertIn('| tdsl-linux-x86_64.tar.gz | 確認不能 |',table)

    def test_no_release_status_does_not_create_or_patch(self):
        with patch.dict(os.environ,{'TARGET_TAG':TAG,'TARGET_SHA':SHA}), patch.object(sys,'argv',['release.py','status']), patch.object(r,'inventory',return_value=(None,'table')), patch.object(r,'gh') as api, patch.object(r,'command') as cmd:
            r.main()
            api.assert_not_called(); cmd.assert_not_called()

    def test_homebrew_mismatched_open_pr_and_api_failure(self):
        assets,proofs = assets_fixture()
        def api(path,*args):
            if path == f'repos/{r.TAP}': return {'default_branch':'main'}
            return [{'head':{'sha':'b'*40},'html_url':'PR'}]
        with patch.object(r,'gh',side_effect=api),patch.object(r,'download',side_effect=proofs.get),patch.object(r,'formula_at',return_value='wrong'):
            with self.assertRaises(r.Unconfirmed): r.homebrew_state(TAG,SHA,assets)
        with patch.object(r,'gh',side_effect=r.Unconfirmed('API down')),patch.object(r,'download',side_effect=proofs.get):
            with self.assertRaises(r.Unconfirmed): r.homebrew_state(TAG,SHA,assets)

    def test_annotated_and_lightweight_event_shas_resolve_to_commit(self):
        for event_sha in [SHA, 'c'*40]:
            def git(*args):
                if args[1] == 'rev-parse': return SHA
                path = args[-1].split(':',1)[1]
                if path == 'Cargo.toml': return '[workspace.package]\nversion="2.1.0"'
                if path.endswith('Cargo.toml'): return '[package]\nversion.workspace=true'
                return json.dumps({'version':'2.1.0','packages':{'':{'version':'2.1.0'}}})
            with patch.object(r,'command',side_effect=git) as command:
                self.assertEqual(r.verify(TAG,event_sha),SHA)
                command.assert_any_call('git','rev-parse','--verify',event_sha+'^{commit}')

    def test_other_workflow_waiting_only_for_inventory_is_not_pending(self):
        def api(path):
            if '/jobs?' in path: return {'jobs':[{'name':'Build VSIX','status':'completed'},{'name':'status / inventory','status':'queued'}]}
            return {'workflow_runs':[{'id':11,'display_title':f'VS Code {TAG}','status':'in_progress'}]}
        with patch.object(r,'gh',side_effect=api):
            self.assertFalse(r.active_release(TAG,10))


if __name__ == '__main__':
    unittest.main()
