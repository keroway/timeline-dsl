#!/usr/bin/env python3
"""Release verification, resumable publishing and live distribution inventory.

Network failures and unprovable provenance are never treated as absence/success.
Only the assets/status subcommands write to GitHub. Registry publication is kept
in workflow jobs so a failed destination remains a failed job.
"""
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import gzip
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tomllib
import urllib.error
import urllib.request
import zipfile

REPO = 'keroway/timeline-dsl'
TAP = 'keroway/homebrew-tap'
CRATES = ('tdsl-parser', 'tdsl-wikidata', 'tdsl-core', 'tdsl-render')
CLI = ('tdsl-linux-x86_64.tar.gz', 'tdsl-linux-aarch64.tar.gz',
       'tdsl-macos-x86_64.tar.gz', 'tdsl-macos-aarch64.tar.gz', 'tdsl-windows-x86_64.zip')
START = '<!-- tdsl-distribution:start -->'
END = '<!-- tdsl-distribution:end -->'
MAX_DOWNLOAD_BYTES = 128 * 1024 * 1024


class Unconfirmed(RuntimeError):
    pass


def command(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def gh(path, method='GET', data=None):
    args = ['gh', 'api', path, '--method', method]
    if data is not None:
        args += ['--input', '-']
    proc = subprocess.run(args, input=json.dumps(data) if data is not None else None,
                          text=True, capture_output=True)
    if proc.returncode:
        # gh reports HTTP status in stderr. Do not classify transport/auth errors as 404.
        if '(HTTP 404)' in proc.stderr:
            return None
        raise Unconfirmed('GitHub API request failed')
    return json.loads(proc.stdout) if proc.stdout else None


def download(url):
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'timeline-dsl-release-inventory'})
        with urllib.request.urlopen(req, timeout=30) as response:
            data = response.read(MAX_DOWNLOAD_BYTES + 1)
            if len(data) > MAX_DOWNLOAD_BYTES:
                raise Unconfirmed('Distribution exceeds verification size limit')
            encoding = response.headers.get('Content-Encoding', 'identity').strip().lower()
            if encoding == 'gzip':
                with gzip.GzipFile(fileobj=io.BytesIO(data)) as stream:
                    data = stream.read(MAX_DOWNLOAD_BYTES + 1)
                if len(data) > MAX_DOWNLOAD_BYTES:
                    raise Unconfirmed('Decoded distribution exceeds verification size limit')
            elif encoding != 'identity':
                raise Unconfirmed('Unsupported distribution content encoding')
            return data
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise Unconfirmed('Registry HTTP request failed') from error
    except (OSError, ValueError, EOFError) as error:
        raise Unconfirmed('Registry request failed') from error


def validate_tag(tag):
    if not re.fullmatch(r'v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', tag):
        raise ValueError('Expected an existing stable tag vX.Y.Z')
    return tag[1:]


def verify(tag, expected_sha=None):
    version = validate_tag(tag)
    sha = command('git', 'rev-parse', '--verify', f'refs/tags/{tag}^{{commit}}')
    if expected_sha:
        if not re.fullmatch('[0-9a-f]{40}', expected_sha):
            raise ValueError('Invalid event SHA')
        # Both lightweight and annotated tag events resolve to a commit.
        expected_sha = command('git', 'rev-parse', '--verify', f'{expected_sha}^{{commit}}')
    if not re.fullmatch('[0-9a-f]{40}', sha) or (expected_sha and sha != expected_sha):
        raise ValueError('Tag SHA mismatch')
    def file(name):
        return command('git', 'show', f'{sha}:{name}')
    cargo = tomllib.loads(file('Cargo.toml'))
    if cargo['workspace']['package']['version'] != version:
        raise ValueError('Cargo workspace version mismatch')
    for name in (*CRATES, 'tdsl-wasm'):
        package = tomllib.loads(file(f'crates/{name}/Cargo.toml'))['package']
        value = package['version']
        if value != version and value != {'workspace': True}:
            raise ValueError(f'{name} version mismatch')
    extension = json.loads(file('editors/vscode/package.json'))
    lock = json.loads(file('editors/vscode/package-lock.json'))
    if extension['version'] != version or lock['version'] != version or lock['packages']['']['version'] != version:
        raise ValueError('VSIX version mismatch')
    return sha


def output(key, value):
    print(f'{key}={value}')
    if os.environ.get('GITHUB_OUTPUT'):
        with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as stream:
            stream.write(f'{key}={value}\n')


def assert_identity(package, version, sha):
    if package.get('version') != version or package.get('sha') != sha:
        raise Unconfirmed('Published distribution provenance does not match tag/SHA')


def tar_json(data, name):
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        stream = archive.extractfile(name)
        if stream is None:
            raise Unconfirmed('Missing provenance file')
        return json.load(stream)


def registry_present(channel, tag, sha):
    version = validate_tag(tag)
    if channel == 'npm':
        metadata = download(f'https://registry.npmjs.org/@keroway%2Ftdsl-wasm/{version}')
        if metadata is None:
            return False
        metadata = json.loads(metadata)
        if metadata.get('name') != '@keroway/tdsl-wasm' or metadata.get('version') != version:
            raise Unconfirmed('npm package identity mismatch')
        url = metadata['dist']['tarball']
        if not url.startswith('https://registry.npmjs.org/'):
            raise Unconfirmed('Unexpected npm tarball host')
        data = download(url)
        if data is None:
            raise Unconfirmed('npm tarball unavailable')
        package = tar_json(data, 'package/package.json')
        if package.get('name') != '@keroway/tdsl-wasm' or package.get('version') != version:
            raise Unconfirmed('npm tarball identity mismatch')
        integrity = metadata['dist'].get('integrity')
        if integrity != 'sha512-' + base64.b64encode(hashlib.sha512(data).digest()).decode():
            raise Unconfirmed('npm tarball integrity mismatch')
        assert_identity(tar_json(data, 'package/release-source.json'), version, sha)
    elif channel in CRATES:
        metadata = download(f'https://crates.io/api/v1/crates/{channel}/{version}')
        if metadata is None:
            return False
        metadata = json.loads(metadata)['version']
        if metadata['num'] != version or metadata['crate'] != channel or metadata.get('yanked'):
            raise Unconfirmed('Crate identity mismatch or yanked')
        data = download(f'https://static.crates.io/crates/{channel}/{channel}-{version}.crate')
        if data is None or hashlib.sha256(data).hexdigest() != metadata['checksum']:
            raise Unconfirmed('Crate checksum mismatch')
        vcs = tar_json(data, f'{channel}-{version}/.cargo_vcs_info.json')
        if vcs['git']['sha1'] != sha or vcs['git'].get('dirty'):
            raise Unconfirmed('Crate source SHA mismatch')
    elif channel == 'Marketplace':
        data = download(f'https://marketplace.visualstudio.com/_apis/public/gallery/publishers/keroway/vsextensions/timeline-dsl/{version}/vspackage')
        if data is None:
            return False
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            manifest = json.loads(archive.read('extension/package.json'))
            if (manifest['publisher'], manifest['name'], manifest['version']) != ('keroway', 'timeline-dsl', version):
                raise Unconfirmed('Marketplace extension identity mismatch')
            assert_identity(json.loads(archive.read('extension/release-source.json')), version, sha)
    else:
        raise ValueError('Unknown registry channel')
    return True


def release(tag):
    return gh(f'repos/{REPO}/releases/tags/{tag}')


def assets_for(release_data):
    if release_data is None:
        return {}
    assets = {}
    for page in range(1, 101):
        items = gh(f'repos/{REPO}/releases/{release_data["id"]}/assets?per_page=100&page={page}')
        if items is None:
            raise Unconfirmed('Release assets unavailable')
        assets.update({item['name']: item for item in items})
        if len(items) < 100:
            return assets
    raise Unconfirmed('Too many release assets')


def asset_present(assets, name, tag, sha):
    if name not in assets:
        return False
    proof = assets.get(name + '.provenance.json')
    if not proof:
        raise Unconfirmed('Asset has no source/checksum provenance')
    raw = download(proof['browser_download_url'])
    if raw is None:
        raise Unconfirmed('Asset provenance unavailable')
    identity = json.loads(raw)
    assert_identity(identity, tag[1:], sha)
    if identity.get('name') != name:
        raise Unconfirmed('Asset filename mismatch')
    digest = assets[name].get('digest')
    if not digest:
        raw_asset = download(assets[name]['browser_download_url'])
        if raw_asset is None:
            raise Unconfirmed('Release asset unavailable')
        digest = 'sha256:' + hashlib.sha256(raw_asset).hexdigest()
    if digest != 'sha256:' + identity['sha256']:
        raise Unconfirmed('Release asset checksum mismatch')
    return True


def publish_assets(tag, sha, files):
    info = release(tag)
    if info is None:
        command('gh', 'release', 'create', tag, '--repo', REPO, '--verify-tag', '--generate-notes')
        info = release(tag)
    assets = assets_for(info)
    for filename in files:
        path = Path(filename)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        proof_path = path.with_name(path.name + '.provenance.json')
        proof = {'version': tag[1:], 'sha': sha, 'name': path.name, 'sha256': digest}
        if path.name in assets:
            # Recover interruption between uploading bytes and uploading proof,
            # but only when the freshly built bytes match the existing digest.
            if proof_path.name not in assets:
                remote_digest = assets[path.name].get('digest')
                if remote_digest != 'sha256:' + digest:
                    raise Unconfirmed('Existing asset without matching provenance')
            else:
                asset_present(assets, path.name, tag, sha)
                continue
        else:
            command('gh', 'release', 'upload', tag, str(path), '--repo', REPO)
        proof_path.write_text(json.dumps(proof) + '\n')
        command('gh', 'release', 'upload', tag, str(proof_path), '--repo', REPO)


def formula_at(ref):
    data = gh(f'repos/{TAP}/contents/Formula/tdsl.rb?ref={ref}')
    if data is None:
        raise Unconfirmed('Homebrew formula unavailable')
    return base64.b64decode(data['content']).decode()


def formula_matches(text, tag, assets):
    for name in CLI[:4]:
        url = f'https://github.com/{REPO}/releases/download/{tag}/{name}'
        digest = assets.get(name, {}).get('digest', '')
        if not digest.startswith('sha256:'):
            raise Unconfirmed('Cannot verify Homebrew archive checksum')
        if not re.search(re.escape(f'url "{url}"') + r'\s+sha256 "' + re.escape(digest[7:]) + '"', text):
            return False
    return True


def homebrew_state(tag, sha, assets):
    for name in CLI[:4]:
        if not asset_present(assets, name, tag, sha):
            return False, 'CLI archives incomplete'
    repo = gh(f'repos/{TAP}')
    if repo is None:
        raise Unconfirmed('Homebrew repository unavailable')
    if formula_matches(formula_at(repo['default_branch']), tag, assets):
        return True, 'Merged and available from tap'
    prs = gh(f'repos/{TAP}/pulls?state=open&head=keroway:bump-tdsl-{tag[1:]}')
    if prs is None:
        raise Unconfirmed('Homebrew PR lookup unavailable')
    if prs:
        if not formula_matches(formula_at(prs[0]['head']['sha']), tag, assets):
            raise Unconfirmed('Existing Homebrew PR content mismatch')
        return False, f'PR created; merge pending: {prs[0]["html_url"]}'
    return False, 'No matching open PR; not distributed'


def active_release(tag, current_run):
    for workflow, title in [('release.yml', f'Release {tag}'), ('vscode-publish.yml', f'VS Code {tag}')]:
        # Paginate: workflow_dispatch runs have the orchestration branch's SHA,
        # so head_sha alone cannot identify a retry of an older tag.
        for page in range(1, 101):
            result = gh(f'repos/{REPO}/actions/workflows/{workflow}/runs?per_page=100&page={page}')
            if result is None:
                raise Unconfirmed('Workflow runs unavailable')
            runs = result['workflow_runs']
            for run in runs:
                if str(run['id']) == str(current_run):
                    continue
                matches = run.get('display_title') == title or run.get('head_branch') == tag
                if matches and run['status'] != 'completed':
                    jobs = []
                    for job_page in range(1, 101):
                        response = gh(f'repos/{REPO}/actions/runs/{run["id"]}/jobs?per_page=100&page={job_page}')
                        if response is None:
                            raise Unconfirmed('Workflow jobs unavailable')
                        jobs.extend(response['jobs'])
                        if len(response['jobs']) < 100:
                            break
                    else:
                        raise Unconfirmed('Workflow jobs pagination limit reached')
                    # An aggregator queued behind this job is not publication
                    # still in progress. Otherwise the last table can stay pending.
                    if not jobs or any(job['status'] != 'completed' and job['name'].split('/')[0].strip() != 'status' for job in jobs):
                        return True
            if len(runs) < 100:
                break
        else:
            raise Unconfirmed('Workflow history pagination limit reached')
    return False


def status_label(value, active):
    if value is True:
        return '公開済み'
    if value is None or active is None:
        return '確認不能'
    return '処理中' if active else '未完了'


def merge_body(body, table):
    block = f'{START}\n{table}\n{END}'
    if START not in body and END not in body:
        return body.rstrip() + '\n\n' + block + '\n'
    if body.count(START) != 1 or body.count(END) != 1 or body.index(START) > body.index(END):
        raise ValueError('Malformed managed release status block')
    begin, rest = body.split(START)
    _, end = rest.split(END)
    return begin + block + end


def inventory(tag, sha, current_run):
    info = None
    try:
        info = release(tag)
        assets = assets_for(info)
    except Exception:
        assets = None
    try:
        active = active_release(tag, current_run)
    except Exception:
        active = None
    rows = []
    for name in (*CLI, f'tdsl-wasm-{tag}.tar.gz', f'timeline-dsl-{tag[1:]}.vsix', 'npm', *CRATES, 'Marketplace', 'Homebrew'):
        detail = 'Verified tag/SHA and distribution'
        try:
            if name == 'Homebrew':
                if assets is None:
                    raise Unconfirmed('Release assets unavailable')
                value, detail = homebrew_state(tag, sha, assets)
            elif name in ('npm', 'Marketplace', *CRATES):
                value = registry_present(name, tag, sha)
            else:
                if assets is None:
                    raise Unconfirmed('Release assets unavailable')
                value = asset_present(assets, name, tag, sha)
            if not value and name != 'Homebrew':
                detail = 'Distribution not found'
        except Exception:
            value, detail = None, 'Verification failed or provenance unavailable; retry verification'
        rows.append((name, status_label(value, active), detail))
    table = f'### Distribution status — {tag}\n\nSource SHA: `{sha}`; checked at {datetime.now(timezone.utc).isoformat()}\n\n| Destination | Status | Evidence |\n|---|---|---|\n'
    table += '\n'.join(f'| {name} | {state} | {detail} |' for name, state, detail in rows)
    return info, table


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['verify', 'stamp', 'pending', 'assets', 'status', 'homebrew'])
    parser.add_argument('files', nargs='*')
    args = parser.parse_args()
    tag = os.environ['TARGET_TAG']
    version = validate_tag(tag)
    if args.action == 'verify':
        sha = verify(tag, os.environ.get('EXPECTED_SHA') or None)
        output('sha', sha); output('tag', tag); output('version', version)
        return
    sha = os.environ['TARGET_SHA']
    if not re.fullmatch('[0-9a-f]{40}', sha):
        raise ValueError('Invalid source SHA')
    if args.action == 'stamp':
        package = Path(args.files[0])
        manifest = json.loads((package / 'package.json').read_text())
        if manifest.get('name') not in ('@keroway/tdsl-wasm', 'timeline-dsl'):
            raise ValueError('Unexpected npm/VSIX package name')
        if manifest['version'] != version:
            raise ValueError('Built npm/VSIX version does not match tag')
        (package / 'release-source.json').write_text(json.dumps({'version': version, 'sha': sha}) + '\n')
        if manifest['name'] == '@keroway/tdsl-wasm':
            manifest.setdefault('files', []).append('release-source.json')
            manifest['publishConfig'] = {'access': 'public'}
            (package / 'package.json').write_text(json.dumps(manifest, indent=2) + '\n')
    elif args.action == 'pending':
        pending = [name for name in args.files if not registry_present(name, tag, sha)]
        output('packages', ' '.join(pending)); output('pending', str(bool(pending)).lower())
    elif args.action == 'assets':
        publish_assets(tag, sha, args.files)
    elif args.action == 'status':
        info, table = inventory(tag, sha, os.environ.get('GITHUB_RUN_ID'))
        print(table)
        if os.environ.get('GITHUB_STEP_SUMMARY'):
            with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as stream:
                stream.write(table + '\n')
        if info is not None:
            latest = release(tag)
            if latest is None:
                raise Unconfirmed('Release disappeared during reconciliation')
            gh(f'repos/{REPO}/releases/{latest["id"]}', 'PATCH', {'body': merge_body(latest.get('body') or '', table)})
    elif args.action == 'homebrew':
        assets = assets_for(release(tag))
        done, detail = homebrew_state(tag, sha, assets)
        output('skip', str(done or detail.startswith('PR created')).lower())
        print(detail)
        if not done:
            for name, key in zip(CLI[:4], ['SHA_LINUX', 'SHA_LINUX_ARM', 'SHA_MACOS_X86', 'SHA_MACOS_ARM']):
                output(key, assets[name]['digest'].removeprefix('sha256:'))


if __name__ == '__main__':
    main()
