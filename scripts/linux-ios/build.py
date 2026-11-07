#!/usr/bin/env python3
"""Experimental Linux iOS build for this repository, without Xcode's build tools."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shlex
import shutil
import subprocess
import sys
import time
import urllib.request
import zipfile

HELPER_REV = '47b0a5e04f2e701d1a16591e1d4457e25e41d0ff'
MIN_IOS = '15.0'
MODULES = ['AppMain', 'SwiftRs', 'Tauri', 'tauri_plugin_dialog',
           'tauri_plugin_file_actions', 'tauri_plugin_opener']


def run(cmd, cwd=None, capture=False):
    print('+ ' + shlex.join(map(str, cmd)), flush=True)
    return subprocess.run(list(map(str, cmd)), cwd=cwd, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None).stdout


def bootstrap(directory):
    dest = directory / 'tauri_xtool'
    dest.mkdir(parents=True, exist_ok=True)
    sums = json.loads(Path(__file__).with_name('helper-sha256.json').read_text())
    for name, digest in sums.items():
        path = dest / name
        if not path.exists() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            url = (f'https://gitlab.com/to-be-continuous/tauri/-/raw/{HELPER_REV}'
                   f'/tools/tauri-xtool/tauri_xtool/{name}')
            print('Downloading pinned helper: ' + name, flush=True)
            data = urllib.request.urlopen(url, timeout=90).read()
            if hashlib.sha256(data).hexdigest() != digest:
                raise RuntimeError('Helper digest mismatch: ' + name)
            path.write_bytes(data)
    sys.path.insert(0, str(directory))


def isolated_sdk(work, sdk, libtool):
    config = work / 'config'
    bundle = config / 'swiftpm/swift-sdks/jm-linux-darwin.artifactbundle'
    bundle.mkdir(parents=True, exist_ok=True)
    for name in ['Developer', 'swift-sdk.json', 'toolset.json']:
        dest = bundle / name
        if dest.is_symlink():
            dest.unlink()
        elif dest.exists():
            raise RuntimeError(f'Refusing to replace non-symlink: {dest}')
        dest.symlink_to(sdk / name)
    info = json.loads((sdk / 'info.json').read_text())
    if len(info['artifacts']) != 1:
        raise RuntimeError('Expected a single-artifact SDK bundle')
    entry = next(iter(info['artifacts'].values()))
    if len(entry.get('variants', [])) != 1:
        raise RuntimeError('Expected a single-variant SDK bundle')
    entry['variants'][0]['path'] = '.'
    info['artifacts'] = {'jm-linux-darwin': entry}
    (bundle / 'info.json').write_text(json.dumps(info, indent=2))
    tools = work / 'bin'
    tools.mkdir(exist_ok=True)
    link = tools / 'libtool'
    if link.is_symlink():
        link.unlink()
    elif link.exists():
        raise RuntimeError(f'Refusing to replace {link}')
    link.symlink_to(libtool)
    os.environ['PATH'] = str(tools) + os.pathsep + os.environ['PATH']
    os.environ['XDG_CONFIG_HOME'] = str(config)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sdk-bundle', type=Path, required=True)
    parser.add_argument('--ld64', type=Path, required=True, help='Linux ld64.lld with iOS support')
    parser.add_argument('--libtool', type=Path, required=True, help='Linux llvm-libtool-darwin')
    parser.add_argument('--work-dir', type=Path)
    parser.add_argument('--rust-target', type=Path)
    parser.add_argument('--skip-install', action='store_true', help='Only if pnpm frozen install already succeeded')
    args = parser.parse_args()
    for name in ['sdk_bundle', 'ld64', 'libtool', 'work_dir', 'rust_target']:
        value = getattr(args, name)
        if value is not None:
            setattr(args, name, value.absolute())
    root = Path(__file__).resolve().parents[2]
    os.chdir(root)
    work = (args.work_dir or root / 'src-tauri/target/linux-ios').resolve()
    work.mkdir(parents=True, exist_ok=True)
    previous = work / 'output'
    if previous.exists():
        previous.rename(work / ('previous-output-' + str(time.time_ns())))
    sdk = args.sdk_bundle.resolve()
    # lld selects the Mach-O driver by argv[0]; preserve the ld64.lld symlink name.
    ld64, libtool = [p.absolute() for p in (args.ld64, args.libtool)]
    if not sdk.is_dir() or not ld64.is_file() or not libtool.is_file():
        parser.error('SDK/tool path does not exist')
    if run(['pnpm', '--version'], capture=True).strip() != '9.5.0':
        parser.error('Use pnpm 9.5.0, matching package.json')
    os.environ['CI'] = 'true'
    if not args.skip_install:
        run(['pnpm', 'install', '--frozen-lockfile'], root)
    bootstrap(work / 'helper')
    from tauri_xtool.rust import run_frontend, build_rust, find_sdk_root
    from tauri_xtool.generate import generate
    config = work / 'project.json'
    file_plist = work / 'FileSharing.plist'
    # Match the original macOS workflow's additional Info.plist values.
    file_plist.write_bytes(plistlib.dumps({
        'UIFileSharingEnabled': True, 'LSSupportsOpeningDocumentsInPlace': True,
        'NSPhotoLibraryAddUsageDescription': '我们需要保存漫画到您的相册',
        'NSPhotoLibraryUsageDescription': '我们需要访问相册以保存漫画',
    }))
    config.write_text(json.dumps({
        'minimumSystemVersion': MIN_IOS,
        # Locked tao 0.33 has no TaoSceneDelegate. Do not invent a scene delegate.
        'sceneManifest': False,
        'infoPlists': ['src-tauri/Info.ios.plist', str(file_plist)],
    }))
    run_frontend(root, str(config))
    rust_target = (args.rust_target or work / 'rust-target').resolve()
    lib, native = build_rust(root, rust_target, sdk, project_config_path=str(config))
    generated = work / 'generated'
    summary = generate(root, generated, lib, native, work / 'cache',
                       project_config_path=str(config))
    isolated_sdk(work, sdk, libtool)
    run(['swift', 'build', '--build-system', 'native', '--swift-sdk', 'jm-linux-darwin',
         '--triple', 'arm64-apple-ios', '-c', 'release', '--product', summary['product']], generated)
    build = generated / '.build/arm64-apple-ios/release'
    if list(build.rglob('*.bundle')):
        raise RuntimeError('Native resource bundles need explicit packaging support')
    if sorted(p.name for p in (generated / 'Resources').iterdir()) != ['PkgInfo']:
        raise RuntimeError('Additional app resources need explicit packaging support')
    objects = []
    for module in MODULES:
        found = sorted((build / (module + '.build')).glob('*.o'))
        if not found:
            raise RuntimeError('No object files for expected module: ' + module)
        objects.extend(found)
    sdkroot = find_sdk_root(sdk)
    sdkver = json.loads((sdkroot / 'SDKSettings.json').read_text())['Version']
    runtime = sdk / 'Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib'
    crts = sorted({p.resolve() for p in runtime.glob('clang/*/lib/darwin/libclang_rt.ios.a')})
    if len(crts) != 1:
        raise RuntimeError('Expected one Apple iOS compiler-rt archive')
    output = work / 'output'
    app = output / 'Payload' / (summary['product'] + '.app')
    if app.exists():
        shutil.rmtree(app)  # only this script's generated bundle
    app.mkdir(parents=True)
    exe = app / summary['product']
    cmd = ['swiftc', '-emit-executable', '-target', 'arm64-apple-ios' + MIN_IOS,
           '-sdk', str(sdkroot), '-resource-dir', str(runtime / 'swift'),
           '-use-ld=' + str(ld64), '-o', str(exe), *map(str, objects), str(lib),
           *shlex.split(native.read_text()), str(crts[0]),
           '-Xlinker', '-platform_version', '-Xlinker', 'ios', '-Xlinker', MIN_IOS,
           '-Xlinker', str(sdkver), '-Xlinker', '-no_adhoc_codesign']
    run(cmd)
    info = plistlib.loads((generated / 'Info.plist').read_bytes())
    info.update(CFBundleIdentifier=summary['bundleId'], CFBundleExecutable=summary['product'],
                CFBundlePackageType='APPL', MinimumOSVersion=MIN_IOS,
                CFBundleSupportedPlatforms=['iPhoneOS'], UIDeviceFamily=[1, 2])
    (app / 'Info.plist').write_bytes(plistlib.dumps(info, fmt=plistlib.FMT_BINARY))
    (app / 'PkgInfo').write_bytes(b'APPL????')
    symbols = run(['llvm-nm', '--defined-only', exe], capture=True)
    required = ['_main', '_start_app', '_init_plugin_file_actions', '_init_plugin_dialog', '_init_plugin_opener']
    for symbol in required:
        if not re.search(r'\s' + re.escape(symbol) + r'$', symbols, re.M):
            raise RuntimeError('Missing native entry: ' + symbol)
    if 'StartupRotation' not in symbols or 'FileActionsPlugin' not in symbols:
        raise RuntimeError('Missing custom native implementation')
    headers = run(['llvm-objdump', '--macho', '--private-headers', exe], capture=True)
    if 'platform ios' not in headers or not re.search(r'minos\s+15\.0', headers):
        raise RuntimeError('Unexpected platform/minimum OS')
    dylibs = run(['llvm-objdump', '--macho', '--dylibs-used', exe], capture=True)
    dylib_paths = [line.strip().split(' (', 1)[0] for line in dylibs.splitlines()[1:] if line.strip()]
    if not dylib_paths or any(not p.startswith(('/System/Library/Frameworks/', '/usr/lib/')) for p in dylib_paths):
        raise RuntimeError('Unexpected non-system dynamic dependency')
    if 'LC_CODE_SIGNATURE' in headers:
        raise RuntimeError('Expected unsigned executable')
    if exe.read_bytes()[:8] != bytes.fromhex('cffaedfe0c000001'):
        raise RuntimeError('Expected arm64 Mach-O')
    ipa = output / ('jmcomic-downloader_' + summary['version'] + '_linux_unsigned.ipa')
    with zipfile.ZipFile(ipa, 'w', zipfile.ZIP_DEFLATED) as archive:
        for f in sorted(app.rglob('*')):
            if f.is_file():
                archive.write(f, f.relative_to(output))
    with zipfile.ZipFile(ipa) as archive:
        if archive.testzip() is not None:
            raise RuntimeError('IPA CRC test failed')
    sha = hashlib.sha256(ipa.read_bytes()).hexdigest()
    versions = {tool: run([tool, '--version'], capture=True).strip().splitlines()[0]
                for tool in ['rustc', 'swift', 'llvm-nm', 'pnpm']}
    report = {'tool_versions': versions, 'source_base': '82a3fc7a420e9a41c13c2da14bc191278f86a285',
              'sdk_settings_sha256': hashlib.sha256((sdkroot / 'SDKSettings.json').read_bytes()).hexdigest(),
              'ipa': ipa.name, 'bytes': ipa.stat().st_size, 'sha256': sha,
              'minimum_ios': MIN_IOS, 'sdk': sdkver, 'arch': 'arm64',
              'native_entries_checked': required, 'unsigned': True,
              'dynamic_dependencies': dylib_paths,
              'device_tested': False, 'custom_icon_packaged': False,
              'route': 'tauri-xtool generator + SwiftPM native + LLVM ld64.lld'}
    (output / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
    (output / 'SHA256SUMS').write_text(sha + '  ' + ipa.name + '\n')
    (output / 'macho-load-commands.txt').write_text(headers)
    (output / 'dynamic-dependencies.txt').write_text(dylibs)
    print(json.dumps(report, indent=2))
    print('Output: ' + str(ipa))


if __name__ == '__main__':
    main()
